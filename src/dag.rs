use crate::circuit::{Circuit, Gate};
use crate::params::{format_param, parse_param_expr};
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

// ── Lazy-compiled regex patterns ──────────────────────────────────────────────

const PARAM_PAT: &str = r"-?(?:\d*\.?\d*\*?pi(?:/\d+\.?\d*)?|\d+\.?\d*(?:[eE][+\-]?\d+)?)";

fn single_gate_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^(\w+)\s+q\[(\d+)\];?$").unwrap())
}

fn single_gate_param_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        let p = PARAM_PAT;
        Regex::new(&format!(
            r"^(\w+)\s*\(\s*({p}(?:\s*,\s*{p})*)\s*\)\s+q\[(\d+)\];?$"
        ))
        .unwrap()
    })
}

fn two_qubit_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^(\w+)\s+q\[(\d+)\],\s*q\[(\d+)\];?$").unwrap())
}

fn two_qubit_param_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        let p = PARAM_PAT;
        Regex::new(&format!(
            r"^(\w+)\s*\(\s*({p})\s*\)\s+q\[(\d+)\],\s*q\[(\d+)\];?$"
        ))
        .unwrap()
    })
}

fn three_qubit_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"^(\w+)\s+q\[(\d+)\],\s*q\[(\d+)\],\s*q\[(\d+)\];?$").unwrap()
    })
}

fn measure_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"^measure\s+q\[(\d+)\]\s*->\s*(\w+)\[(\d+)\];?$").unwrap()
    })
}

fn reset_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^reset\s+q\[(\d+)\];?$").unwrap())
}

fn if_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"^if\s*\(\s*(\w+)(?:\[(\d+)\])?\s*==\s*(\d+)\s*\)\s+(\w+)\s+q\[(\d+)\];?$")
            .unwrap()
    })
}

fn if_param_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        let p = PARAM_PAT;
        Regex::new(&format!(
            r"^if\s*\(\s*(\w+)(?:\[(\d+)\])?\s*==\s*(\d+)\s*\)\s+(\w+)\s*\(\s*({p})\s*\)\s+q\[(\d+)\];?$"
        ))
        .unwrap()
    })
}

fn qreg_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"qreg\s+(\w+)\[(\d+)\]").unwrap())
}

fn creg_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"creg\s+(\w+)\[(\d+)\]").unwrap())
}

fn noise_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        let p = PARAM_PAT;
        Regex::new(&format!(
            r"^//\s*noise\s+(\w+)\s+q\[(\d+)\](?:\s+param=({p}))?$"
        ))
        .unwrap()
    })
}

fn barrier_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^barrier\s+").unwrap())
}

// ── Data structures ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct DAGNode {
    pub id: String,
    pub type_name: String,
    pub target: isize,
    pub control: isize,
    pub controls: Vec<usize>,
    pub measure_source: isize,
    pub step: isize,
    pub params: Vec<f64>,
    pub is_dagger: bool,
    pub is_reset: bool,
    pub classical_control: isize,
    pub is_noise: bool,
    pub noise_type: String,
    pub dependencies: Vec<String>,
}

impl Default for DAGNode {
    fn default() -> Self {
        DAGNode {
            id: String::new(),
            type_name: String::new(),
            target: -1,
            control: -1,
            controls: vec![],
            measure_source: -1,
            step: 0,
            params: vec![],
            is_dagger: false,
            is_reset: false,
            classical_control: -1,
            is_noise: false,
            noise_type: String::new(),
            dependencies: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub struct CircuitDAG {
    pub nodes: HashMap<String, DAGNode>,
    pub num_qubits: usize,
    pub num_cbits: usize,
    root_nodes: Vec<String>,
}

impl Default for CircuitDAG {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitDAG {
    pub fn new() -> Self {
        CircuitDAG {
            nodes: HashMap::new(),
            num_qubits: 0,
            num_cbits: 0,
            root_nodes: vec![],
        }
    }

    fn generate_node_id(gate_type: &str, target: isize, step: isize) -> String {
        format!("{gate_type}_q{target}_s{step}")
    }

    pub fn add_node(&mut self, mut node: DAGNode) {
        if node.id.is_empty() {
            node.id = Self::generate_node_id(&node.type_name, node.target, node.step);
        }

        // Update qubit count
        let max_qubit = {
            let mut m = if node.target >= 0 { node.target as usize } else { 0 };
            if node.control >= 0 {
                m = m.max(node.control as usize);
            }
            for &c in &node.controls {
                m = m.max(c);
            }
            if node.measure_source >= 0 {
                m = m.max(node.measure_source as usize);
            }
            m
        };
        if max_qubit + 1 > self.num_qubits {
            self.num_qubits = max_qubit + 1;
        }

        // Update classical bit count
        if node.classical_control >= 0 {
            let needed = node.classical_control as usize + 1;
            if needed > self.num_cbits {
                self.num_cbits = needed;
            }
        }
        if node.type_name == "MEASURE" && node.target >= 0 {
            let needed = node.target as usize + 1;
            if needed > self.num_cbits {
                self.num_cbits = needed;
            }
        }
        if node.measure_source >= 0 {
            let needed = node.measure_source as usize + 1;
            if needed > self.num_cbits {
                self.num_cbits = needed;
            }
        }

        self.nodes.insert(node.id.clone(), node);
        self.update_root_nodes();
    }

    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.remove(node_id);
        let id_owned = node_id.to_string();
        for node in self.nodes.values_mut() {
            node.dependencies.retain(|d| d != &id_owned);
        }
        self.update_root_nodes();
    }

    fn update_root_nodes(&mut self) {
        self.root_nodes = self
            .nodes
            .iter()
            .filter(|(_, n)| n.dependencies.is_empty())
            .map(|(id, _)| id.clone())
            .collect();
    }

    pub fn topological_sort(&self) -> Vec<&DAGNode> {
        let mut visited = HashMap::new();
        let mut result: Vec<&DAGNode> = Vec::new();

        fn visit<'a>(
            node_id: &str,
            nodes: &'a HashMap<String, DAGNode>,
            visited: &mut HashMap<String, bool>,
            result: &mut Vec<&'a DAGNode>,
        ) {
            if visited.contains_key(node_id) {
                return;
            }
            visited.insert(node_id.to_string(), true);
            if let Some(node) = nodes.get(node_id) {
                for dep in &node.dependencies {
                    visit(dep, nodes, visited, result);
                }
                result.push(node);
            }
        }

        let root_ids: Vec<String> = self.root_nodes.clone();
        for root_id in &root_ids {
            visit(root_id, &self.nodes, &mut visited, &mut result);
        }
        let all_ids: Vec<String> = self.nodes.keys().cloned().collect();
        for id in &all_ids {
            visit(id, &self.nodes, &mut visited, &mut result);
        }
        result
    }

    pub fn max_step(&self) -> isize {
        self.nodes.values().map(|n| n.step).max().unwrap_or(0)
    }

    pub fn to_circuit(&self) -> Circuit {
        let mut circuit = Circuit {
            num_qubits: self.num_qubits,
            gates: Vec::new(),
            max_steps: self.max_step() as usize,
        };

        for node in self.nodes.values() {
            let gate = Gate {
                step: node.step,
                type_name: node.type_name.clone(),
                target: if node.target >= 0 { node.target as usize } else { 0 },
                control: node.control,
                controls: node.controls.clone(),
                measure_source: node.measure_source,
                params: node.params.clone(),
                is_dagger: node.is_dagger,
                is_reset: node.is_reset,
                classical_control: node.classical_control,
                is_noise: node.is_noise,
                noise_type: node.noise_type.clone(),
            };
            circuit.gates.push(gate);
        }

        circuit
    }

    pub fn to_qasm(&self) -> String {
        let mut nodes: Vec<&DAGNode> = self.topological_sort();
        nodes.sort_by_key(|n| n.step);

        // Determine qubit and cbit counts
        let num_qubits = {
            let max_q = nodes.iter().fold(self.num_qubits as isize - 1, |acc, n| {
                let mut m = acc.max(n.target).max(n.control).max(n.measure_source);
                for &c in &n.controls {
                    m = m.max(c as isize);
                }
                m
            });
            (max_q + 1).max(self.num_qubits as isize).max(1) as usize
        };

        let num_cbits = {
            let max_c = nodes.iter().fold(self.num_cbits as isize - 1, |acc, n| {
                let mut m = acc;
                if n.type_name == "MEASURE" {
                    m = m.max(n.target);
                }
                if n.measure_source >= 0 {
                    m = m.max(n.measure_source);
                }
                if n.classical_control >= 0 {
                    m = m.max(n.classical_control);
                }
                m
            });
            (max_c + 1).max(1) as usize
        };

        let mut sb = String::new();
        sb.push_str("OPENQASM 2.0;\n");
        sb.push_str("include \"qelib1.inc\";\n\n");
        sb.push_str(&format!("qreg q[{num_qubits}];\n"));
        sb.push_str(&format!("creg c[{num_cbits}];\n\n"));

        // Group by step
        let max_step = nodes.iter().map(|n| n.step).max().unwrap_or(0);
        let mut step_map: HashMap<isize, Vec<&DAGNode>> = HashMap::new();
        for node in &nodes {
            step_map.entry(node.step).or_default().push(node);
        }

        for step in 0..=max_step {
            if let Some(step_nodes) = step_map.get(&step) {
                for node in step_nodes {
                    sb.push_str(&write_node_qasm(node, num_qubits));
                }
            }
        }

        sb
    }

    // ── Gate placement helpers ────────────────────────────────────────────────

    pub fn get_node_at(&self, step: isize, qubit: usize) -> Option<&DAGNode> {
        let q = qubit as isize;
        self.nodes.values().find(|n| {
            n.step == step
                && (n.target == q
                    || n.control == q
                    || n.measure_source == q
                    || n.controls.contains(&qubit))
        })
    }

    pub fn get_node_at_mut(&mut self, step: isize, qubit: usize) -> Option<String> {
        let q = qubit as isize;
        self.nodes
            .iter()
            .find(|(_, n)| {
                n.step == step
                    && (n.target == q
                        || n.control == q
                        || n.measure_source == q
                        || n.controls.contains(&qubit))
            })
            .map(|(id, _)| id.clone())
    }

    pub fn can_place_gate_at(&self, step: isize, qubits: &[usize]) -> bool {
        for &qubit in qubits {
            if let Some(node) = self.get_node_at(step, qubit) {
                if node.type_name == "BARRIER" {
                    return false;
                }
                if node.control >= 0
                    || !node.controls.is_empty()
                    || node.measure_source >= 0
                {
                    return false;
                }
            }
        }
        true
    }

    pub fn remove_node_at(&mut self, step: isize, qubit: usize) {
        if let Some(id) = self.get_node_at_mut(step, qubit) {
            self.remove_node(&id);
        }
    }

    pub fn remove_nodes_on_qubit(&mut self, qubit: usize) {
        let q = qubit as isize;
        let to_remove: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, n)| {
                n.target == q
                    || n.control == q
                    || n.measure_source == q
                    || n.controls.contains(&qubit)
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in to_remove {
            self.remove_node(&id);
        }
    }

    // ─── Add helpers (mirrors dag.go) ─────────────────────────────────────────

    fn build_deps(&self, qubits_used: &[usize], step: isize, gate_type: &str) -> Vec<String> {
        let mut last_gate_on_qubit: HashMap<usize, String> = HashMap::new();
        for n in self.nodes.values() {
            let mut qs = vec![];
            if n.target >= 0 {
                qs.push(n.target as usize);
            }
            if n.control >= 0 {
                qs.push(n.control as usize);
            }
            for &c in &n.controls {
                qs.push(c);
            }
            if n.measure_source >= 0 {
                qs.push(n.measure_source as usize);
            }
            for q in qs {
                if n.step < step || (n.step == step && n.type_name.as_str() < gate_type) {
                    last_gate_on_qubit.insert(q, n.id.clone());
                }
            }
        }
        let mut dep_set: HashMap<String, bool> = HashMap::new();
        for &q in qubits_used {
            if let Some(id) = last_gate_on_qubit.get(&q) {
                dep_set.insert(id.clone(), true);
            }
        }
        dep_set.into_keys().collect()
    }

    pub fn add_gate(&mut self, gate_type: &str, target: usize, step: isize, control: Option<usize>) {
        let ctrl = control.map(|c| c as isize).unwrap_or(-1);
        let qubits = if ctrl >= 0 {
            vec![target, ctrl as usize]
        } else {
            vec![target]
        };
        let deps = self.build_deps(&qubits, step, gate_type);
        let id = Self::generate_node_id(gate_type, target as isize, step);
        self.add_node(DAGNode {
            id,
            type_name: gate_type.to_string(),
            target: target as isize,
            control: ctrl,
            step,
            dependencies: deps,
            ..Default::default()
        });
    }

    pub fn add_parameterized_gate(
        &mut self,
        gate_type: &str,
        target: usize,
        step: isize,
        params: Vec<f64>,
        control: Option<usize>,
    ) {
        let ctrl = control.map(|c| c as isize).unwrap_or(-1);
        let qubits = if ctrl >= 0 {
            vec![target, ctrl as usize]
        } else {
            vec![target]
        };
        let deps = self.build_deps(&qubits, step, gate_type);
        let id = Self::generate_node_id(gate_type, target as isize, step);
        self.add_node(DAGNode {
            id,
            type_name: gate_type.to_string(),
            target: target as isize,
            control: ctrl,
            step,
            params,
            dependencies: deps,
            ..Default::default()
        });
    }

    pub fn add_multi_control_gate(
        &mut self,
        gate_type: &str,
        target: usize,
        step: isize,
        controls: Vec<usize>,
    ) {
        let mut qubits = vec![target];
        qubits.extend_from_slice(&controls);
        let deps = self.build_deps(&qubits, step, gate_type);
        let id = Self::generate_node_id(gate_type, target as isize, step);
        self.add_node(DAGNode {
            id,
            type_name: gate_type.to_string(),
            target: target as isize,
            controls,
            step,
            dependencies: deps,
            ..Default::default()
        });
    }

    pub fn add_classical_control_gate(
        &mut self,
        gate_type: &str,
        target: usize,
        step: isize,
        cbit: usize,
    ) {
        if cbit + 1 > self.num_cbits {
            self.num_cbits = cbit + 1;
        }
        let deps = self.build_deps(&[target], step, gate_type);
        let id = Self::generate_node_id(gate_type, target as isize, step);
        self.add_node(DAGNode {
            id,
            type_name: gate_type.to_string(),
            target: target as isize,
            step,
            classical_control: cbit as isize,
            dependencies: deps,
            ..Default::default()
        });
    }

    pub fn add_dagger_gate(&mut self, gate_type: &str, target: usize, step: isize) {
        let deps = self.build_deps(&[target], step, gate_type);
        let id = Self::generate_node_id(gate_type, target as isize, step);
        self.add_node(DAGNode {
            id,
            type_name: gate_type.to_string(),
            target: target as isize,
            step,
            is_dagger: true,
            dependencies: deps,
            ..Default::default()
        });
    }

    pub fn add_reset(&mut self, target: usize, step: isize) {
        let deps = self.build_deps(&[target], step, "RESET");
        let id = Self::generate_node_id("RESET", target as isize, step);
        self.add_node(DAGNode {
            id,
            type_name: "RESET".to_string(),
            target: target as isize,
            step,
            is_reset: true,
            dependencies: deps,
            ..Default::default()
        });
    }

    pub fn add_noise(
        &mut self,
        target: usize,
        step: isize,
        noise_type: &str,
        params: Vec<f64>,
    ) {
        let deps = self.build_deps(&[target], step, "NOISE");
        let id = Self::generate_node_id("NOISE", target as isize, step);
        self.add_node(DAGNode {
            id,
            type_name: "NOISE".to_string(),
            target: target as isize,
            step,
            params,
            is_noise: true,
            noise_type: noise_type.to_string(),
            dependencies: deps,
            ..Default::default()
        });
    }

    pub fn add_measure_control_gate(&mut self, source: usize, target: usize, step: isize) {
        if source + 1 > self.num_cbits {
            self.num_cbits = source + 1;
        }
        let qubits = [source, target];
        let deps = self.build_deps(&qubits, step, "MCX");
        let id = Self::generate_node_id("MCX", target as isize, step);
        self.add_node(DAGNode {
            id,
            type_name: "MCX".to_string(),
            target: target as isize,
            measure_source: source as isize,
            step,
            dependencies: deps,
            ..Default::default()
        });
    }

    pub fn add_barrier(&mut self, step: isize) {
        // Remove existing barrier at this step
        let to_remove: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.step == step && n.type_name == "BARRIER")
            .map(|(id, _)| id.clone())
            .collect();
        for id in to_remove {
            self.remove_node(&id);
        }
        let id = Self::generate_node_id("BARRIER", -1, step);
        self.add_node(DAGNode {
            id,
            type_name: "BARRIER".to_string(),
            step,
            ..Default::default()
        });
    }

    // ── QASM Parsing ──────────────────────────────────────────────────────────

    pub fn parse_qasm(&mut self, qasm: &str) -> Result<(), String> {
        self.nodes.clear();
        self.root_nodes.clear();

        let lines: Vec<&str> = qasm.lines().collect();
        let mut creg_map: HashMap<String, usize> = HashMap::new();
        let mut creg_offset: usize = 0;

        let resolve_cbit = |reg_name: &str, bit_idx: &str, creg_map: &HashMap<String, usize>| -> usize {
            if let Some(&start) = creg_map.get(reg_name) {
                if !bit_idx.is_empty() {
                    let offset: usize = bit_idx.parse().unwrap_or(0);
                    return start + offset;
                }
                return start;
            }
            // fallback: try to parse c[N] style
            if reg_name.starts_with('c') {
                if let Ok(idx) = reg_name[1..].parse::<usize>() {
                    return idx;
                }
            }
            0
        };

        let mut last_gate_on_qubit: HashMap<usize, String> = HashMap::new();
        let mut current_step_qubits: HashMap<usize, bool> = HashMap::new();
        let mut current_step: isize = 0;

        let get_qubits_used = |node: &DAGNode| -> Vec<usize> {
            let mut qs = vec![];
            if node.target >= 0 {
                qs.push(node.target as usize);
            }
            if node.control >= 0 {
                qs.push(node.control as usize);
            }
            for &c in &node.controls {
                qs.push(c);
            }
            if node.measure_source >= 0 {
                qs.push(node.measure_source as usize);
            }
            qs
        };

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            i += 1;

            if line.is_empty() {
                continue;
            }

            // Comments / noise
            if line.starts_with("//") {
                if let Some(caps) = noise_re().captures(line) {
                    let target: usize = caps[2].parse().unwrap_or(0);
                    let qubits_used = vec![target];
                    for &q in &qubits_used {
                        if *current_step_qubits.get(&q).unwrap_or(&false) {
                            current_step += 1;
                            current_step_qubits.clear();
                            break;
                        }
                    }
                    for &q in &qubits_used {
                        current_step_qubits.insert(q, true);
                    }
                    let noise_type = caps[1].to_string();
                    let params = if let Some(m) = caps.get(3) {
                        parse_param_expr(m.as_str())
                            .map(|v| vec![v])
                            .unwrap_or_default()
                    } else {
                        vec![]
                    };
                    let mut node = DAGNode {
                        type_name: "NOISE".to_string(),
                        target: target as isize,
                        step: current_step,
                        params,
                        is_noise: true,
                        noise_type,
                        ..Default::default()
                    };
                    if let Some(last_id) = last_gate_on_qubit.get(&target) {
                        node.dependencies.push(last_id.clone());
                    }
                    node.id = Self::generate_node_id("NOISE", target as isize, current_step);
                    let node_id = node.id.clone();
                    self.add_node(node);
                    last_gate_on_qubit.insert(target, node_id);
                    current_step += 1;
                    current_step_qubits.clear();
                }
                continue;
            }

            if line.starts_with("OPENQASM") || line.starts_with("include") {
                continue;
            }

            if line.starts_with("qreg") {
                if let Some(caps) = qreg_re().captures(line) {
                    let n: usize = caps[2].parse().unwrap_or(0);
                    self.num_qubits = n;
                }
                continue;
            }

            if line.starts_with("creg") {
                if let Some(caps) = creg_re().captures(line) {
                    let reg_name = caps[1].to_string();
                    let reg_size: usize = caps[2].parse().unwrap_or(0);
                    creg_map.insert(reg_name, creg_offset);
                    creg_offset += reg_size;
                }
                continue;
            }

            // Parse gate line
            let node_opt = parse_gate_line(
                line,
                &lines,
                &mut i,
                &creg_map,
                &resolve_cbit,
            );

            if let Some(mut node) = node_opt {
                let qubits_used = get_qubits_used(&node);

                // Barriers always start a new step
                if node.type_name == "BARRIER" {
                    if !current_step_qubits.is_empty() {
                        current_step += 1;
                        current_step_qubits.clear();
                    }
                    node.step = current_step;
                    current_step += 1;
                    current_step_qubits.clear();
                } else {
                    let conflict = qubits_used
                        .iter()
                        .any(|q| *current_step_qubits.get(q).unwrap_or(&false));

                    // Multi-qubit gates always start a new step
                    if node.control >= 0 || !node.controls.is_empty() || node.measure_source >= 0 {
                        if !current_step_qubits.is_empty() {
                            current_step += 1;
                            current_step_qubits.clear();
                        }
                    } else if conflict {
                        current_step += 1;
                        current_step_qubits.clear();
                    }

                    node.step = current_step;
                    for &q in &qubits_used {
                        current_step_qubits.insert(q, true);
                    }
                }

                // Establish dependencies
                let mut dep_set: HashMap<String, bool> = HashMap::new();
                for &qubit in &qubits_used {
                    if let Some(last_id) = last_gate_on_qubit.get(&qubit) {
                        dep_set.insert(last_id.clone(), true);
                    }
                }
                for dep_id in dep_set.into_keys() {
                    node.dependencies.push(dep_id);
                }

                node.id = Self::generate_node_id(&node.type_name, node.target, node.step);
                let node_id = node.id.clone();
                for &qubit in &qubits_used {
                    last_gate_on_qubit.insert(qubit, node_id.clone());
                }
                self.add_node(node);
            }
        }

        Ok(())
    }

    pub fn clone_dag(&self) -> Self {
        self.clone()
    }
}

// ── QASM node writer ──────────────────────────────────────────────────────────

fn write_node_qasm(node: &DAGNode, num_qubits: usize) -> String {
    let mut s = String::new();

    if node.type_name == "BARRIER" {
        let qubits: Vec<String> = (0..num_qubits).map(|q| format!("q[{q}]")).collect();
        s.push_str(&format!("barrier {};\n", qubits.join(", ")));
    } else if node.is_noise {
        if !node.params.is_empty() {
            s.push_str(&format!(
                "// noise {} q[{}] param={}\n",
                node.noise_type,
                node.target,
                format_param(node.params[0])
            ));
        } else {
            s.push_str(&format!("// noise {} q[{}]\n", node.noise_type, node.target));
        }
    } else if node.is_reset {
        s.push_str(&format!("reset q[{}];\n", node.target));
    } else if node.classical_control >= 0 {
        if node.control >= 0 {
            s.push_str(&format!(
                "if (c[{}]==1) cx q[{}], q[{}];\n",
                node.classical_control, node.control, node.target
            ));
        } else if !node.controls.is_empty() {
            let gate_type = node.type_name.to_lowercase();
            let ctrl_strs: Vec<String> = node.controls.iter().map(|c| format!("q[{c}]")).collect();
            s.push_str(&format!(
                "if (c[{}]==1) {} {}, q[{}];\n",
                node.classical_control,
                gate_type,
                ctrl_strs.join(", "),
                node.target
            ));
        } else {
            let gate_type = node.type_name.to_lowercase();
            if !node.params.is_empty() {
                s.push_str(&format!(
                    "if (c[{}]==1) {}({}) q[{}];\n",
                    node.classical_control,
                    gate_type,
                    format_param(node.params[0]),
                    node.target
                ));
            } else if node.is_dagger {
                s.push_str(&format!(
                    "if (c[{}]==1) {}dg q[{}];\n",
                    node.classical_control, gate_type, node.target
                ));
            } else {
                s.push_str(&format!(
                    "if (c[{}]==1) {} q[{}];\n",
                    node.classical_control, gate_type, node.target
                ));
            }
        }
    } else if node.measure_source >= 0 {
        s.push_str(&format!(
            "measure q[{}] -> c[{}];\n",
            node.measure_source, node.measure_source
        ));
        s.push_str(&format!(
            "if (c[{}]==1) x q[{}];\n",
            node.measure_source, node.target
        ));
    } else if node.type_name == "MEASURE" {
        s.push_str(&format!("measure q[{}] -> c[{}];\n", node.target, node.target));
    } else if !node.controls.is_empty() {
        match node.type_name.as_str() {
            "CCX" | "TOFFOLI" if node.controls.len() >= 2 => {
                s.push_str(&format!(
                    "ccx q[{}], q[{}], q[{}];\n",
                    node.controls[0], node.controls[1], node.target
                ));
            }
            _ => {
                let gate_type = node.type_name.to_lowercase();
                let ctrl_strs: Vec<String> = node.controls.iter().map(|c| format!("q[{c}]")).collect();
                s.push_str(&format!(
                    "{} {}, q[{}];\n",
                    gate_type,
                    ctrl_strs.join(", "),
                    node.target
                ));
            }
        }
    } else if node.control >= 0 {
        match node.type_name.as_str() {
            "CX" => s.push_str(&format!("cx q[{}], q[{}];\n", node.control, node.target)),
            "CZ" => s.push_str(&format!("cz q[{}], q[{}];\n", node.control, node.target)),
            "SWAP" => s.push_str(&format!("swap q[{}], q[{}];\n", node.control, node.target)),
            "CH" => s.push_str(&format!("ch q[{}], q[{}];\n", node.control, node.target)),
            "CRX" if !node.params.is_empty() => s.push_str(&format!(
                "crx({}) q[{}], q[{}];\n",
                format_param(node.params[0]),
                node.control,
                node.target
            )),
            "CRY" if !node.params.is_empty() => s.push_str(&format!(
                "cry({}) q[{}], q[{}];\n",
                format_param(node.params[0]),
                node.control,
                node.target
            )),
            "CRZ" if !node.params.is_empty() => s.push_str(&format!(
                "crz({}) q[{}], q[{}];\n",
                format_param(node.params[0]),
                node.control,
                node.target
            )),
            "CP" | "CU1" if !node.params.is_empty() => s.push_str(&format!(
                "cu1({}) q[{}], q[{}];\n",
                format_param(node.params[0]),
                node.control,
                node.target
            )),
            _ => s.push_str(&format!("cx q[{}], q[{}];\n", node.control, node.target)),
        }
    } else {
        let gate_type = node.type_name.to_lowercase();
        match gate_type.as_str() {
            "rx" | "ry" | "rz" | "p" | "u1" => {
                if node.params.len() == 1 {
                    s.push_str(&format!(
                        "{}({}) q[{}];\n",
                        gate_type,
                        format_param(node.params[0]),
                        node.target
                    ));
                }
            }
            "u2" => {
                if node.params.len() == 2 {
                    s.push_str(&format!(
                        "u2({}, {}) q[{}];\n",
                        format_param(node.params[0]),
                        format_param(node.params[1]),
                        node.target
                    ));
                }
            }
            "u3" => {
                if node.params.len() == 3 {
                    s.push_str(&format!(
                        "u3({}, {}, {}) q[{}];\n",
                        format_param(node.params[0]),
                        format_param(node.params[1]),
                        format_param(node.params[2]),
                        node.target
                    ));
                }
            }
            "s" | "t" | "sx" | "sy" | "sz" => {
                if node.is_dagger {
                    s.push_str(&format!("{}dg q[{}];\n", gate_type, node.target));
                } else {
                    s.push_str(&format!("{} q[{}];\n", gate_type, node.target));
                }
            }
            _ => {
                s.push_str(&format!("{} q[{}];\n", gate_type, node.target));
            }
        }
    }

    s
}

// ── Gate line parser ──────────────────────────────────────────────────────────

fn parse_gate_line(
    line: &str,
    lines: &[&str],
    idx: &mut usize,
    creg_map: &HashMap<String, usize>,
    resolve_cbit: &dyn Fn(&str, &str, &HashMap<String, usize>) -> usize,
) -> Option<DAGNode> {
    // Reset
    if let Some(caps) = reset_re().captures(line) {
        let target: usize = caps[1].parse().unwrap_or(0);
        return Some(DAGNode {
            type_name: "RESET".to_string(),
            target: target as isize,
            is_reset: true,
            ..Default::default()
        });
    }

    // Barrier
    if barrier_re().is_match(line) {
        return Some(DAGNode {
            type_name: "BARRIER".to_string(),
            ..Default::default()
        });
    }

    // Measurement (with MCX detection)
    if let Some(caps) = measure_re().captures(line) {
        let source: usize = caps[1].parse().unwrap_or(0);
        let cbit = resolve_cbit(&caps[2], &caps[3], creg_map);

        // Look ahead for MCX pattern
        if *idx < lines.len() {
            let next_line = lines[*idx].trim();
            if let Some(if_caps) = if_re().captures(next_line) {
                let cond_bit = resolve_cbit(&if_caps[1], if_caps.get(2).map_or("", |m| m.as_str()), creg_map);
                let target: usize = if_caps[5].parse().unwrap_or(0);
                if cond_bit == cbit {
                    *idx += 1;
                    return Some(DAGNode {
                        type_name: "MCX".to_string(),
                        target: target as isize,
                        measure_source: source as isize,
                        ..Default::default()
                    });
                }
            }
        }

        return Some(DAGNode {
            type_name: "MEASURE".to_string(),
            target: source as isize,
            ..Default::default()
        });
    }

    // Classically-controlled parameterized gate
    if let Some(caps) = if_param_re().captures(line) {
        let cbit = resolve_cbit(&caps[1], caps.get(2).map_or("", |m| m.as_str()), creg_map);
        let gate_type = caps[4].to_uppercase();
        let param = parse_param_expr(&caps[5]).unwrap_or(0.0);
        let target: usize = caps[6].parse().unwrap_or(0);
        return Some(DAGNode {
            type_name: gate_type,
            target: target as isize,
            params: vec![param],
            classical_control: cbit as isize,
            ..Default::default()
        });
    }

    // Classically-controlled gate
    if let Some(caps) = if_re().captures(line) {
        let cbit = resolve_cbit(&caps[1], caps.get(2).map_or("", |m| m.as_str()), creg_map);
        let gate_type = caps[4].to_uppercase();
        let target: usize = caps[5].parse().unwrap_or(0);
        return Some(DAGNode {
            type_name: gate_type,
            target: target as isize,
            classical_control: cbit as isize,
            ..Default::default()
        });
    }

    // Three-qubit gates
    if let Some(caps) = three_qubit_re().captures(line) {
        let gate_type = caps[1].to_uppercase();
        let q1: usize = caps[2].parse().unwrap_or(0);
        let q2: usize = caps[3].parse().unwrap_or(0);
        let q3: usize = caps[4].parse().unwrap_or(0);
        return Some(DAGNode {
            type_name: gate_type,
            target: q3 as isize,
            controls: vec![q1, q2],
            ..Default::default()
        });
    }

    // Two-qubit parameterized
    if let Some(caps) = two_qubit_param_re().captures(line) {
        let gate_type = caps[1].to_uppercase();
        let param = parse_param_expr(&caps[2]).unwrap_or(0.0);
        let q1: usize = caps[3].parse().unwrap_or(0);
        let q2: usize = caps[4].parse().unwrap_or(0);
        return Some(DAGNode {
            type_name: gate_type,
            target: q2 as isize,
            control: q1 as isize,
            params: vec![param],
            ..Default::default()
        });
    }

    // Two-qubit gate
    if let Some(caps) = two_qubit_re().captures(line) {
        let gate_type = caps[1].to_uppercase();
        let q1: usize = caps[2].parse().unwrap_or(0);
        let q2: usize = caps[3].parse().unwrap_or(0);
        return Some(DAGNode {
            type_name: gate_type,
            target: q2 as isize,
            control: q1 as isize,
            ..Default::default()
        });
    }

    // Single-qubit parameterized
    if let Some(caps) = single_gate_param_re().captures(line) {
        let gate_type = caps[1].to_uppercase();
        let params_str = caps[2].to_string();
        let target: usize = caps[3].parse().unwrap_or(0);
        let params: Vec<f64> = params_str
            .split(',')
            .filter_map(|s| parse_param_expr(s.trim()))
            .collect();
        return Some(DAGNode {
            type_name: gate_type,
            target: target as isize,
            params,
            ..Default::default()
        });
    }

    // Single-qubit gate (including dagger)
    if let Some(caps) = single_gate_re().captures(line) {
        let mut gate_type = caps[1].to_uppercase();
        let target: usize = caps[2].parse().unwrap_or(0);

        let mut is_dagger = false;
        if gate_type.ends_with("DG") {
            is_dagger = true;
            gate_type = gate_type[..gate_type.len() - 2].to_string();
        }

        return Some(DAGNode {
            type_name: gate_type,
            target: target as isize,
            is_dagger,
            ..Default::default()
        });
    }

    None
}
