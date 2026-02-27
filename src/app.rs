use crate::circuit::Gate;
use crate::dag::CircuitDAG;
use crate::menu::is_parameterized_gate;
use crate::params::{format_param, parse_params};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    Circuit,
    Qasm,
    Menu,
    SelectTarget,
    InputParam,
    SelectControls,
    EditGate,
    EditParam,
    EditTarget,
    EditControl,
}

#[derive(Clone, Debug)]
pub struct EditOption {
    pub label: String,
    pub action: &'static str,
    pub ctrl_idx: isize, // -1 for the single Control field
}

pub struct App {
    pub dag: CircuitDAG,
    pub cursor_qubit: usize,
    pub cursor_step: isize,
    pub width: u16,
    pub height: u16,
    pub focus: Focus,
    pub status_msg: String,

    // QASM editor state
    pub qasm_text: String,
    pub last_qasm: String,
    pub qasm_cursor: usize,  // byte offset into qasm_text
    pub qasm_scroll: u16,   // vertical scroll offset (lines)


    // Menu state
    pub menu_cat: usize,
    pub menu_item: usize,

    // Gate placement pending state
    pub pending_gate: String,
    pub target_qubit: usize,
    pub param_input: String,
    pub control_qubits: Vec<usize>,

    // Edit gate state
    pub edit_gate: Option<Gate>,
    pub edit_menu_idx: usize,
    pub edit_orig_step: isize,
    pub edit_control_idx: isize,

    // State panel view toggle
    pub show_statevector: bool,
}

impl App {
    pub fn new() -> Self {
        let mut dag = CircuitDAG::new();
        dag.num_qubits = 4;

        let mut app = App {
            dag,
            cursor_qubit: 0,
            cursor_step: 0,
            width: 80,
            height: 24,
            focus: Focus::Circuit,
            status_msg: String::new(),
            qasm_text: String::new(),
            last_qasm: String::new(),
            qasm_cursor: 0,
            qasm_scroll: 0,
            menu_cat: 0,
            menu_item: 0,
            pending_gate: String::new(),
            target_qubit: 0,
            param_input: String::new(),
            control_qubits: vec![],
            edit_gate: None,
            edit_menu_idx: 0,
            edit_orig_step: 0,
            edit_control_idx: -1,
            show_statevector: false,
        };
        app.sync_from_dag();
        app
    }

    pub fn sync_from_dag(&mut self) {
        let qasm = self.dag.to_qasm();
        self.qasm_text = qasm.clone();
        self.last_qasm = qasm;
        self.qasm_cursor = self.qasm_text.len();
        self.qasm_scroll = 0;
    }

    pub fn parse_qasm_input(&mut self) {
        if self.qasm_text != self.last_qasm {
            let mut new_dag = CircuitDAG::new();
            if new_dag.parse_qasm(&self.qasm_text).is_ok() {
                self.dag = new_dag;
                self.last_qasm = self.qasm_text.clone();
            }
        }
    }

    pub fn circuit(&self) -> crate::circuit::Circuit {
        self.dag.to_circuit()
    }

    pub fn place_gate(&mut self, gate_type: &str, target_q: isize) -> bool {
        let qubits_needed: Option<Vec<usize>> = match gate_type {
            "CX" | "CZ" | "SWAP" | "CH" | "CRX" | "CRY" | "CRZ" | "CU1" => {
                if target_q < 0 {
                    return false;
                }
                Some(vec![self.cursor_qubit, target_q as usize])
            }
            "CCX" => {
                if target_q < 0 {
                    return false;
                }
                let mut qs = vec![self.cursor_qubit, target_q as usize];
                qs.extend_from_slice(&self.control_qubits);
                Some(qs)
            }
            "MCX" => {
                if target_q < 0 {
                    return false;
                }
                Some(vec![self.cursor_qubit, target_q as usize])
            }
            "BARRIER" => None,
            _ => Some(vec![self.cursor_qubit]),
        };

        if let Some(ref qs) = qubits_needed {
            if !self.dag.can_place_gate_at(self.cursor_step, qs) {
                self.status_msg =
                    "Cannot place: qubit already used by another gate at this step".to_string();
                self.param_input.clear();
                self.control_qubits.clear();
                self.pending_gate.clear();
                return false;
            }
        }

        // Remove existing gates
        if let Some(ref qs) = qubits_needed {
            for &q in qs {
                self.dag.remove_node_at(self.cursor_step, q);
            }
        }

        let params: Vec<f64> = if !self.param_input.is_empty() {
            parse_params(&self.param_input).unwrap_or_default()
        } else {
            vec![]
        };

        match gate_type {
            "CX" | "CZ" | "SWAP" | "CH" | "CRX" | "CRY" | "CRZ" | "CU1" => {
                let tq = target_q as usize;
                if !params.is_empty() {
                    self.dag.add_parameterized_gate(
                        gate_type,
                        tq,
                        self.cursor_step,
                        params,
                        Some(self.cursor_qubit),
                    );
                } else {
                    self.dag.add_gate(gate_type, tq, self.cursor_step, Some(self.cursor_qubit));
                }
            }
            "CCX" => {
                let tq = target_q as usize;
                let mut controls = vec![self.cursor_qubit];
                if !self.control_qubits.is_empty() {
                    controls.extend_from_slice(&self.control_qubits);
                    for &cq in &self.control_qubits.clone() {
                        self.dag.remove_node_at(self.cursor_step, cq);
                    }
                }
                self.dag
                    .add_multi_control_gate("CCX", tq, self.cursor_step, controls);
            }
            "MCX" => {
                let tq = target_q as usize;
                self.dag
                    .add_measure_control_gate(self.cursor_qubit, tq, self.cursor_step);
            }
            "MEASURE" => {
                self.dag.add_gate("MEASURE", self.cursor_qubit, self.cursor_step, None);
            }
            "BARRIER" => {
                self.dag.add_barrier(self.cursor_step);
            }
            "RESET" => {
                self.dag.add_reset(self.cursor_qubit, self.cursor_step);
            }
            "RX" | "RY" | "RZ" | "P" | "U1" => {
                let p = if !params.is_empty() { params } else { vec![0.0] };
                self.dag.add_parameterized_gate(
                    gate_type,
                    self.cursor_qubit,
                    self.cursor_step,
                    p,
                    None,
                );
            }
            "U2" => {
                let mut p = params;
                while p.len() < 2 {
                    p.push(0.0);
                }
                self.dag.add_parameterized_gate(
                    "U2",
                    self.cursor_qubit,
                    self.cursor_step,
                    p[..2].to_vec(),
                    None,
                );
            }
            "U3" => {
                let mut p = params;
                while p.len() < 3 {
                    p.push(0.0);
                }
                self.dag.add_parameterized_gate(
                    "U3",
                    self.cursor_qubit,
                    self.cursor_step,
                    p[..3].to_vec(),
                    None,
                );
            }
            "SDG" | "TDG" => {
                let base = &gate_type[..gate_type.len() - 2];
                self.dag.add_dagger_gate(base, self.cursor_qubit, self.cursor_step);
            }
            "NOISE_DEPOL" | "NOISE_AMP" | "NOISE_PHASE" => {
                let noise_type = match gate_type {
                    "NOISE_DEPOL" => "depolarizing",
                    "NOISE_AMP" => "amplitude_damping",
                    _ => "phase_damping",
                };
                let p = if !params.is_empty() { params } else { vec![0.01] };
                self.dag
                    .add_noise(self.cursor_qubit, self.cursor_step, noise_type, p);
            }
            _ => {
                self.dag
                    .add_gate(gate_type, self.cursor_qubit, self.cursor_step, None);
            }
        }

        self.param_input.clear();
        self.control_qubits.clear();
        self.pending_gate.clear();
        self.cursor_step += 1;
        self.sync_from_dag();
        true
    }

    pub fn get_edit_options(&self) -> Vec<EditOption> {
        let gate = match &self.edit_gate {
            Some(g) => g,
            None => return vec![],
        };
        let mut opts = vec![];

        if !gate.params.is_empty() || is_parameterized_gate(&gate.type_name) {
            let param_str = if gate.params.is_empty() {
                "none".to_string()
            } else {
                gate.params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        if i == 0 {
                            format_param(*p)
                        } else {
                            format!(", {}", format_param(*p))
                        }
                    })
                    .collect::<String>()
            };
            opts.push(EditOption {
                label: format!("Parameters: {param_str}"),
                action: "edit_param",
                ctrl_idx: -1,
            });
        }

        opts.push(EditOption {
            label: format!("Target: q[{}]", gate.target),
            action: "edit_target",
            ctrl_idx: -1,
        });

        if gate.control >= 0 {
            opts.push(EditOption {
                label: format!("Control: q[{}]", gate.control),
                action: "edit_control",
                ctrl_idx: -1,
            });
        }
        for (i, &ctrl) in gate.controls.iter().enumerate() {
            opts.push(EditOption {
                label: format!("Control {}: q[{ctrl}]", i + 1),
                action: "edit_control",
                ctrl_idx: i as isize,
            });
        }

        opts.push(EditOption {
            label: "Delete gate".to_string(),
            action: "delete",
            ctrl_idx: -1,
        });

        opts
    }

    pub fn handle_char_input(&mut self, ch: char) {
        if matches!(ch, '0'..='9' | '.' | ',' | '-' | 'e' | 'E' | '+' | 'p' | 'i' | '*' | '/') {
            self.param_input.push(ch);
        }
    }

    pub fn qasm_insert_char(&mut self, ch: char) {
        self.qasm_text.insert(self.qasm_cursor, ch);
        self.qasm_cursor += ch.len_utf8();
    }

    pub fn qasm_backspace(&mut self) {
        if self.qasm_cursor == 0 { return; }
        let mut pos = self.qasm_cursor - 1;
        while pos > 0 && !self.qasm_text.is_char_boundary(pos) { pos -= 1; }
        self.qasm_text.remove(pos);
        self.qasm_cursor = pos;
    }

    pub fn qasm_delete_forward(&mut self) {
        if self.qasm_cursor >= self.qasm_text.len() { return; }
        self.qasm_text.remove(self.qasm_cursor);
    }

    pub fn qasm_cursor_row_col(&self) -> (usize, usize) {
        let cursor = self.qasm_cursor.min(self.qasm_text.len());
        let before = &self.qasm_text[..cursor];
        let row = before.bytes().filter(|&b| b == b'\n').count();
        let col = match before.rfind('\n') {
            Some(p) => before.len() - p - 1,
            None => before.len(),
        };
        (row, col)
    }

    pub fn qasm_move_left(&mut self) {
        if self.qasm_cursor == 0 { return; }
        let mut pos = self.qasm_cursor - 1;
        while pos > 0 && !self.qasm_text.is_char_boundary(pos) { pos -= 1; }
        self.qasm_cursor = pos;
    }

    pub fn qasm_move_right(&mut self) {
        if self.qasm_cursor >= self.qasm_text.len() { return; }
        let ch = self.qasm_text[self.qasm_cursor..].chars().next().unwrap();
        self.qasm_cursor += ch.len_utf8();
    }

    pub fn qasm_move_up(&mut self) {
        let (row, col) = self.qasm_cursor_row_col();
        if row == 0 { return; }
        let lines: Vec<&str> = self.qasm_text.split('\n').collect();
        let target_col = col.min(lines[row - 1].len());
        let mut off = 0usize;
        for r in 0..(row - 1) { off += lines[r].len() + 1; }
        off += target_col;
        self.qasm_cursor = off;
    }

    pub fn qasm_move_down(&mut self) {
        let (row, col) = self.qasm_cursor_row_col();
        let lines: Vec<&str> = self.qasm_text.split('\n').collect();
        if row + 1 >= lines.len() { return; }
        let target_col = col.min(lines[row + 1].len());
        let mut off = 0usize;
        for r in 0..=row { off += lines[r].len() + 1; }
        off += target_col;
        self.qasm_cursor = off;
    }

    pub fn qasm_move_home(&mut self) {
        let cursor = self.qasm_cursor.min(self.qasm_text.len());
        let before = &self.qasm_text[..cursor];
        self.qasm_cursor = match before.rfind('\n') {
            Some(p) => p + 1,
            None => 0,
        };
    }

    pub fn qasm_move_end(&mut self) {
        let cursor = self.qasm_cursor.min(self.qasm_text.len());
        self.qasm_cursor = self.qasm_text[cursor..]
            .find('\n')
            .map(|p| cursor + p)
            .unwrap_or(self.qasm_text.len());
    }

    pub fn save_circuit(&mut self) -> Result<(), std::io::Error> {
        let qasm = self.dag.to_qasm();
        std::fs::write("circuit.qasm", &qasm)?;
        Ok(())
    }

    pub fn next_available_target(&self, from: usize, direction: isize, excluded: &[usize]) -> Option<usize> {
        let nq = self.dag.num_qubits;
        if direction > 0 {
            for q in (from + 1)..nq {
                if !excluded.contains(&q) {
                    return Some(q);
                }
            }
        } else {
            for q in (0..from).rev() {
                if !excluded.contains(&q) {
                    return Some(q);
                }
            }
        }
        None
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
