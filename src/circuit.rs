#[derive(Clone, Debug, Default)]
pub struct Gate {
    pub step: isize,
    pub type_name: String,
    pub target: usize,
    pub control: isize,
    pub controls: Vec<usize>,
    pub measure_source: isize,
    pub params: Vec<f64>,
    pub is_dagger: bool,
    pub is_reset: bool,
    pub is_noise: bool,
    pub noise_type: String,
    pub classical_control: isize,
}

impl Gate {
    pub fn references(&self, qubit: usize) -> bool {
        let q = qubit as isize;
        self.target == qubit
            || self.control == q
            || self.measure_source == q
            || self.controls.contains(&qubit)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Circuit {
    pub num_qubits: usize,
    pub gates: Vec<Gate>,
    pub max_steps: usize,
}

impl Circuit {
    pub fn num_cbits(&self) -> usize {
        let mut max = -1isize;
        for g in &self.gates {
            if g.type_name == "MEASURE" {
                max = max.max(g.target as isize);
            }
            if g.measure_source >= 0 {
                max = max.max(g.measure_source);
            }
        }
        if max < 0 { 0 } else { max as usize + 1 }
    }

    pub fn get_measure_at_step(&self, step: isize) -> isize {
        for g in &self.gates {
            if g.step != step {
                continue;
            }
            if g.type_name == "MEASURE" {
                return g.target as isize;
            }
            if g.measure_source >= 0 {
                return g.measure_source;
            }
        }
        -1
    }

    pub fn get_gate_at(&self, step: isize, qubit: usize) -> Option<&Gate> {
        self.gates.iter().find(|g| g.step == step && g.references(qubit))
    }

    pub fn remove_gate_at(&mut self, step: isize, qubit: usize) {
        self.gates.retain(|g| {
            if g.step == step && g.type_name == "BARRIER" {
                return false;
            }
            !(g.step == step && g.references(qubit))
        });
    }

    pub fn get_cell_info(&self, step: isize, qubit: usize) -> CellInfo {
        let mut info = CellInfo::default();

        if let Some(gate) = self.get_gate_at(step, qubit) {
            info.gate = Some(gate.clone());
            info.is_control = gate.control == qubit as isize
                || gate.controls.contains(&qubit);
            info.is_target = gate.target == qubit
                && (gate.control >= 0 || !gate.controls.is_empty());
        }

        // Check for barrier
        for g in &self.gates {
            if g.step == step && g.type_name == "BARRIER" {
                info.is_barrier = true;
                if info.gate.is_none() {
                    info.gate = Some(g.clone());
                }
                break;
            }
        }

        // Vertical connections
        for g in &self.gates {
            if g.step != step {
                continue;
            }
            let (min_q, max_q) = if !g.controls.is_empty() {
                let mut mn = g.target;
                let mut mx = g.target;
                for &c in &g.controls {
                    mn = mn.min(c);
                    mx = mx.max(c);
                }
                (mn, mx)
            } else if g.control >= 0 {
                let c = g.control as usize;
                (g.target.min(c), g.target.max(c))
            } else if g.measure_source >= 0 {
                let ms = g.measure_source as usize;
                (g.target.min(ms), g.target.max(ms))
            } else {
                continue;
            };

            if qubit >= min_q && qubit <= max_q {
                if qubit > min_q {
                    info.vert_above = true;
                }
                if qubit < max_q {
                    info.vert_below = true;
                }
                if qubit > min_q && qubit < max_q && info.gate.is_none() {
                    info.pass_through = true;
                }
            }
        }

        // Measurement connections down to classical wire
        for g in &self.gates {
            if g.step != step {
                continue;
            }
            let mq = if g.type_name == "MEASURE" {
                Some(g.target)
            } else if g.measure_source >= 0 {
                Some(g.measure_source as usize)
            } else {
                None
            };
            if let Some(measured) = mq {
                if qubit > measured {
                    info.measure_below = true;
                }
            }
        }

        info
    }
}

#[derive(Clone, Debug, Default)]
pub struct CellInfo {
    pub gate: Option<Gate>,
    pub is_control: bool,
    pub is_target: bool,
    pub vert_above: bool,
    pub vert_below: bool,
    pub pass_through: bool,
    pub measure_below: bool,
    pub is_barrier: bool,
}

