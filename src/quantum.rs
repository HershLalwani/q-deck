use crate::circuit::Circuit;
use num_complex::Complex;
use std::f64::consts::PI;

pub type ComplexF64 = Complex<f64>;

#[derive(Clone, Debug)]
pub struct StateVector {
    pub amplitudes: Vec<ComplexF64>,
    pub num_qubits: usize,
}

impl StateVector {
    pub fn new(num_qubits: usize) -> Self {
        let n = 1 << num_qubits;
        let mut amplitudes = vec![ComplexF64::new(0.0, 0.0); n];
        if n > 0 {
            amplitudes[0] = ComplexF64::new(1.0, 0.0);
        }
        Self {
            amplitudes,
            num_qubits,
        }
    }

    pub fn clone_state(&self) -> Self {
        self.clone()
    }

    pub fn apply_gate(&mut self, gate_type: &str, target: usize, control: isize, params: &[f64]) {
        match gate_type {
            "H" => self.apply_h(target),
            "X" => self.apply_x(target),
            "Y" => self.apply_y(target),
            "Z" => self.apply_z(target),
            "S" => self.apply_s(target, false),
            "SDG" | "Sdg" => self.apply_s(target, true),
            "T" => self.apply_t(target, false),
            "TDG" | "Tdg" => self.apply_t(target, true),
            "RX" => {
                let theta = params.first().copied().unwrap_or(0.0);
                self.apply_rx(target, theta);
            }
            "RY" => {
                let theta = params.first().copied().unwrap_or(0.0);
                self.apply_ry(target, theta);
            }
            "RZ" | "P" | "U1" => {
                let theta = params.first().copied().unwrap_or(0.0);
                self.apply_rz(target, theta);
            }
            "CX" => {
                if control >= 0 {
                    self.apply_cx(control as usize, target);
                }
            }
            "CZ" => {
                if control >= 0 {
                    self.apply_cz(control as usize, target);
                }
            }
            "SWAP" => {
                if control >= 0 {
                    self.apply_swap(control as usize, target);
                }
            }
            "RESET" => self.apply_reset(target),
            "MEASURE" => {}
            _ => {}
        }
    }

    fn apply_h(&mut self, q: usize) {
        let h_factor = ComplexF64::new(1.0 / std::f64::consts::SQRT_2, 0.0);
        let n = self.amplitudes.len();
        let bit = 1 << q;
        let mut new_amps = vec![ComplexF64::new(0.0, 0.0); n];
        for i in 0..n {
            if (i & bit) == 0 {
                let j = i | bit;
                new_amps[i] = h_factor * (self.amplitudes[i] + self.amplitudes[j]);
                new_amps[j] = h_factor * (self.amplitudes[i] - self.amplitudes[j]);
            }
        }
        self.amplitudes = new_amps;
    }

    fn apply_x(&mut self, q: usize) {
        let n = self.amplitudes.len();
        let bit = 1 << q;
        for i in 0..n {
            if (i & bit) == 0 {
                let j = i | bit;
                self.amplitudes.swap(i, j);
            }
        }
    }

    fn apply_y(&mut self, q: usize) {
        let n = self.amplitudes.len();
        let bit = 1 << q;
        let i_comp = ComplexF64::new(0.0, 1.0);
        for i in 0..n {
            if (i & bit) == 0 {
                let j = i | bit;
                let amp_i = self.amplitudes[i];
                let amp_j = self.amplitudes[j];
                self.amplitudes[i] = i_comp * amp_j;
                self.amplitudes[j] = -i_comp * amp_i;
            }
        }
    }

    fn apply_z(&mut self, q: usize) {
        let n = self.amplitudes.len();
        let bit = 1 << q;
        for i in 0..n {
            if (i & bit) != 0 {
                self.amplitudes[i] = -self.amplitudes[i];
            }
        }
    }

    fn apply_s(&mut self, q: usize, dagger: bool) {
        let n = self.amplitudes.len();
        let bit = 1 << q;
        let factor = if dagger {
            ComplexF64::new(0.0, -1.0)
        } else {
            ComplexF64::new(0.0, 1.0)
        };
        for i in 0..n {
            if (i & bit) != 0 {
                self.amplitudes[i] = self.amplitudes[i] * factor;
            }
        }
    }

    fn apply_t(&mut self, q: usize, dagger: bool) {
        let n = self.amplitudes.len();
        let bit = 1 << q;
        let angle = if dagger { -PI / 4.0 } else { PI / 4.0 };
        let factor = ComplexF64::from_polar(1.0, angle);
        for i in 0..n {
            if (i & bit) != 0 {
                self.amplitudes[i] = self.amplitudes[i] * factor;
            }
        }
    }

    fn apply_rx(&mut self, q: usize, theta: f64) {
        let n = self.amplitudes.len();
        let bit = 1 << q;
        let c = ComplexF64::new((theta / 2.0).cos(), 0.0);
        let js = ComplexF64::new(0.0, -(theta / 2.0).sin());
        let mut new_amps = vec![ComplexF64::new(0.0, 0.0); n];
        for i in 0..n {
            if (i & bit) == 0 {
                let j = i | bit;
                new_amps[i] = c * self.amplitudes[i] + js * self.amplitudes[j];
                new_amps[j] = js * self.amplitudes[i] + c * self.amplitudes[j];
            }
        }
        self.amplitudes = new_amps;
    }

    fn apply_ry(&mut self, q: usize, theta: f64) {
        let n = self.amplitudes.len();
        let bit = 1 << q;
        let c = ComplexF64::new((theta / 2.0).cos(), 0.0);
        let s_ = ComplexF64::new((theta / 2.0).sin(), 0.0);
        let mut new_amps = vec![ComplexF64::new(0.0, 0.0); n];
        for i in 0..n {
            if (i & bit) == 0 {
                let j = i | bit;
                new_amps[i] = c * self.amplitudes[i] - s_ * self.amplitudes[j];
                new_amps[j] = s_ * self.amplitudes[i] + c * self.amplitudes[j];
            }
        }
        self.amplitudes = new_amps;
    }

    fn apply_rz(&mut self, q: usize, theta: f64) {
        let n = self.amplitudes.len();
        let bit = 1 << q;
        let phase = ComplexF64::from_polar(1.0, theta / 2.0);
        for i in 0..n {
            if (i & bit) != 0 {
                self.amplitudes[i] = self.amplitudes[i] * phase;
            } else {
                self.amplitudes[i] = self.amplitudes[i] * phase.conj();
            }
        }
    }

    fn apply_cx(&mut self, control: usize, target: usize) {
        let n = self.amplitudes.len();
        let c_bit = 1 << control;
        let t_bit = 1 << target;
        for i in 0..n {
            if (i & c_bit) != 0 && (i & t_bit) == 0 {
                let j = i | t_bit;
                self.amplitudes.swap(i, j);
            }
        }
    }

    fn apply_cz(&mut self, control: usize, target: usize) {
        let n = self.amplitudes.len();
        let c_bit = 1 << control;
        let t_bit = 1 << target;
        for i in 0..n {
            if (i & c_bit) != 0 && (i & t_bit) != 0 {
                self.amplitudes[i] = -self.amplitudes[i];
            }
        }
    }

    fn apply_swap(&mut self, q1: usize, q2: usize) {
        let n = self.amplitudes.len();
        let bit1 = 1 << q1;
        let bit2 = 1 << q2;
        for i in 0..n {
            if (i & bit1) != 0 && (i & bit2) == 0 {
                let j = (i & !bit1) | bit2;
                self.amplitudes.swap(i, j);
            }
        }
    }

    fn apply_reset(&mut self, q: usize) {
        let n = self.amplitudes.len();
        let bit = 1 << q;

        let mut prob0 = 0.0;
        for i in 0..n {
            if (i & bit) == 0 {
                prob0 += self.amplitudes[i].norm_sqr();
            }
        }

        let mut norm = 1.0;
        if prob0 > 0.0 {
            norm = prob0.sqrt();
        }

        for i in 0..n {
            if (i & bit) == 0 {
                self.amplitudes[i] = self.amplitudes[i] / norm;
            } else {
                self.amplitudes[i] = ComplexF64::new(0.0, 0.0);
            }
        }
    }

    pub fn get_qubit_probabilities(&self) -> Vec<QubitProbability> {
        let mut probs = vec![
            QubitProbability {
                prob0: 0.0,
                prob1: 0.0
            };
            self.num_qubits
        ];
        let n = self.amplitudes.len();

        for i in 0..n {
            let prob = self.amplitudes[i].norm_sqr();
            for q in 0..self.num_qubits {
                if (i & (1 << q)) != 0 {
                    probs[q].prob1 += prob;
                } else {
                    probs[q].prob0 += prob;
                }
            }
        }

        probs
    }

    pub fn get_qsphere_states(&self) -> Vec<QSphereState> {
        let mut states = Vec::new();
        let n = self.amplitudes.len();

        for i in 0..n {
            let amp = self.amplitudes[i];
            let prob = amp.norm_sqr();

            if prob > 1e-10 {
                let phase = amp.arg();
                let hamming = i.count_ones() as usize;
                states.push(QSphereState {
                    basis_state: i,
                    amplitude: amp,
                    prob,
                    phase,
                    hamming,
                });
            }
        }

        states
    }
}

#[derive(Clone, Debug, Default)]
pub struct QubitProbability {
    pub prob0: f64,
    pub prob1: f64,
}

#[derive(Clone, Debug)]
pub struct QSphereState {
    pub basis_state: usize,
    pub amplitude: ComplexF64,
    pub prob: f64,
    pub phase: f64,
    pub hamming: usize,
}

pub fn simulate_circuit(circuit: &Circuit, up_to_step: isize) -> StateVector {
    if circuit.num_qubits == 0 {
        return StateVector::new(1);
    }

    let mut state = StateVector::new(circuit.num_qubits);

    let mut gates = circuit.gates.clone();

    // Sort gates by step
    gates.sort_by_key(|g| g.step);

    for gate in gates {
        if up_to_step >= 0 && gate.step > up_to_step {
            continue;
        }

        if gate.type_name == "BARRIER" || gate.type_name == "MEASURE" || gate.type_name == "MCX" {
            continue;
        }
        if gate.is_noise {
            continue;
        }
        if gate.classical_control >= 0 {
            continue;
        }

        if !gate.controls.is_empty() {
            for &ctrl in &gate.controls {
                state.apply_gate(&gate.type_name, gate.target, ctrl as isize, &gate.params);
            }
        } else {
            state.apply_gate(&gate.type_name, gate.target, gate.control, &gate.params);
        }
    }

    state
}
