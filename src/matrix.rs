use crate::circuit::Circuit;
use num_complex::Complex;
use std::f64::consts::{PI, SQRT_2};

pub type C64 = Complex<f64>;

/// A 2^n x 2^n unitary matrix stored in row-major order.
#[derive(Clone, Debug)]
pub struct UnitaryMatrix {
    pub data: Vec<Vec<C64>>,
    pub dim: usize,
}

impl UnitaryMatrix {
    /// Create an identity matrix of given dimension.
    pub fn identity(dim: usize) -> Self {
        let mut data = vec![vec![C64::new(0.0, 0.0); dim]; dim];
        for (i, row) in data.iter_mut().enumerate().take(dim) {
            row[i] = C64::new(1.0, 0.0);
        }
        UnitaryMatrix { data, dim }
    }

    /// Multiply two matrices: self * other.
    pub fn mul(&self, other: &UnitaryMatrix) -> UnitaryMatrix {
        assert_eq!(self.dim, other.dim);
        let n = self.dim;
        let mut result = vec![vec![C64::new(0.0, 0.0); n]; n];
        for (i, row) in result.iter_mut().enumerate().take(n) {
            for (j, cell) in row.iter_mut().enumerate().take(n) {
                let mut sum = C64::new(0.0, 0.0);
                for k in 0..n {
                    sum += self.data[i][k] * other.data[k][j];
                }
                *cell = sum;
            }
        }
        UnitaryMatrix {
            data: result,
            dim: n,
        }
    }

    /// Tensor (Kronecker) product: self ⊗ other.
    pub fn tensor(&self, other: &UnitaryMatrix) -> UnitaryMatrix {
        let n = self.dim * other.dim;
        let mut data = vec![vec![C64::new(0.0, 0.0); n]; n];
        for i in 0..self.dim {
            for j in 0..self.dim {
                for k in 0..other.dim {
                    for l in 0..other.dim {
                        data[i * other.dim + k][j * other.dim + l] =
                            self.data[i][j] * other.data[k][l];
                    }
                }
            }
        }
        UnitaryMatrix { data, dim: n }
    }
}

// ── Single-qubit gate matrices (2x2) ──────────────────────────────────────────

fn mat2(a: C64, b: C64, c: C64, d: C64) -> UnitaryMatrix {
    UnitaryMatrix {
        data: vec![vec![a, b], vec![c, d]],
        dim: 2,
    }
}

fn zero() -> C64 {
    C64::new(0.0, 0.0)
}
fn one() -> C64 {
    C64::new(1.0, 0.0)
}

pub fn gate_matrix_h() -> UnitaryMatrix {
    let h = C64::new(1.0 / SQRT_2, 0.0);
    mat2(h, h, h, -h)
}

pub fn gate_matrix_x() -> UnitaryMatrix {
    mat2(zero(), one(), one(), zero())
}

pub fn gate_matrix_y() -> UnitaryMatrix {
    mat2(zero(), C64::new(0.0, -1.0), C64::new(0.0, 1.0), zero())
}

pub fn gate_matrix_z() -> UnitaryMatrix {
    mat2(one(), zero(), zero(), C64::new(-1.0, 0.0))
}

pub fn gate_matrix_i() -> UnitaryMatrix {
    UnitaryMatrix::identity(2)
}

pub fn gate_matrix_s() -> UnitaryMatrix {
    mat2(one(), zero(), zero(), C64::new(0.0, 1.0))
}

pub fn gate_matrix_sdg() -> UnitaryMatrix {
    mat2(one(), zero(), zero(), C64::new(0.0, -1.0))
}

pub fn gate_matrix_t() -> UnitaryMatrix {
    let phase = C64::from_polar(1.0, PI / 4.0);
    mat2(one(), zero(), zero(), phase)
}

pub fn gate_matrix_tdg() -> UnitaryMatrix {
    let phase = C64::from_polar(1.0, -PI / 4.0);
    mat2(one(), zero(), zero(), phase)
}

pub fn gate_matrix_sx() -> UnitaryMatrix {
    let half = C64::new(0.5, 0.0);
    let hi = C64::new(0.0, 0.5);
    mat2(half + hi, half - hi, half - hi, half + hi)
}

pub fn gate_matrix_sy() -> UnitaryMatrix {
    let half = C64::new(0.5, 0.0);
    let hi = C64::new(0.0, 0.5);
    // SY = sqrt(Y) = (I + iY) / (1+i) — standard convention
    mat2(half + hi, -(half - hi), half - hi, half + hi)
}

pub fn gate_matrix_rx(theta: f64) -> UnitaryMatrix {
    let c = C64::new((theta / 2.0).cos(), 0.0);
    let js = C64::new(0.0, -(theta / 2.0).sin());
    mat2(c, js, js, c)
}

pub fn gate_matrix_ry(theta: f64) -> UnitaryMatrix {
    let c = C64::new((theta / 2.0).cos(), 0.0);
    let s = C64::new((theta / 2.0).sin(), 0.0);
    mat2(c, -s, s, c)
}

pub fn gate_matrix_rz(theta: f64) -> UnitaryMatrix {
    let phase_neg = C64::from_polar(1.0, -theta / 2.0);
    let phase_pos = C64::from_polar(1.0, theta / 2.0);
    mat2(phase_neg, zero(), zero(), phase_pos)
}

pub fn gate_matrix_p(theta: f64) -> UnitaryMatrix {
    let phase = C64::from_polar(1.0, theta);
    mat2(one(), zero(), zero(), phase)
}

pub fn gate_matrix_u1(lambda: f64) -> UnitaryMatrix {
    gate_matrix_p(lambda)
}

pub fn gate_matrix_u2(phi: f64, lambda: f64) -> UnitaryMatrix {
    let h = C64::new(1.0 / SQRT_2, 0.0);
    let e_il = C64::from_polar(1.0, lambda);
    let e_ip = C64::from_polar(1.0, phi);
    let e_ipl = C64::from_polar(1.0, phi + lambda);
    mat2(h, -h * e_il, h * e_ip, h * e_ipl)
}

pub fn gate_matrix_u3(theta: f64, phi: f64, lambda: f64) -> UnitaryMatrix {
    let c = C64::new((theta / 2.0).cos(), 0.0);
    let s = C64::new((theta / 2.0).sin(), 0.0);
    let e_il = C64::from_polar(1.0, lambda);
    let e_ip = C64::from_polar(1.0, phi);
    let e_ipl = C64::from_polar(1.0, phi + lambda);
    mat2(c, -s * e_il, s * e_ip, c * e_ipl)
}

// ── Two-qubit gate matrices (4x4) ────────────────────────────────────────────

fn mat4(rows: [[C64; 4]; 4]) -> UnitaryMatrix {
    UnitaryMatrix {
        data: rows.iter().map(|r| r.to_vec()).collect(),
        dim: 4,
    }
}

pub fn gate_matrix_cx() -> UnitaryMatrix {
    let o = zero();
    let i = one();
    mat4([[i, o, o, o], [o, i, o, o], [o, o, o, i], [o, o, i, o]])
}

pub fn gate_matrix_cz() -> UnitaryMatrix {
    let o = zero();
    let i = one();
    mat4([[i, o, o, o], [o, i, o, o], [o, o, i, o], [o, o, o, -i]])
}

pub fn gate_matrix_swap() -> UnitaryMatrix {
    let o = zero();
    let i = one();
    mat4([[i, o, o, o], [o, o, i, o], [o, i, o, o], [o, o, o, i]])
}

pub fn gate_matrix_ch() -> UnitaryMatrix {
    let o = zero();
    let i = one();
    let h = C64::new(1.0 / SQRT_2, 0.0);
    mat4([[i, o, o, o], [o, i, o, o], [o, o, h, h], [o, o, h, -h]])
}

/// Build a controlled-U gate matrix from a 2x2 U.
/// |0><0| ⊗ I + |1><1| ⊗ U
fn controlled_gate(u: &UnitaryMatrix) -> UnitaryMatrix {
    assert_eq!(u.dim, 2);
    let o = zero();
    let i = one();
    mat4([
        [i, o, o, o],
        [o, i, o, o],
        [o, o, u.data[0][0], u.data[0][1]],
        [o, o, u.data[1][0], u.data[1][1]],
    ])
}

pub fn gate_matrix_crx(theta: f64) -> UnitaryMatrix {
    controlled_gate(&gate_matrix_rx(theta))
}

pub fn gate_matrix_cry(theta: f64) -> UnitaryMatrix {
    controlled_gate(&gate_matrix_ry(theta))
}

pub fn gate_matrix_crz(theta: f64) -> UnitaryMatrix {
    controlled_gate(&gate_matrix_rz(theta))
}

pub fn gate_matrix_cu1(lambda: f64) -> UnitaryMatrix {
    controlled_gate(&gate_matrix_u1(lambda))
}

// ── Three-qubit gate matrices (8x8) ──────────────────────────────────────────

pub fn gate_matrix_ccx() -> UnitaryMatrix {
    let n = 8;
    let mut m = UnitaryMatrix::identity(n);
    // CCX (Toffoli): swaps |110⟩ and |111⟩
    // In our basis ordering: |q2 q1 q0⟩
    // Control on q2, q1; target q0
    // Swap rows/cols 6 (110) and 7 (111)
    m.data[6][6] = zero();
    m.data[6][7] = one();
    m.data[7][6] = one();
    m.data[7][7] = zero();
    m
}

// ── Build full circuit unitary ───────────────────────────────────────────────

/// Get the 2x2 matrix for a single-qubit gate type.
fn single_qubit_matrix(gate_type: &str, params: &[f64], is_dagger: bool) -> Option<UnitaryMatrix> {
    let m = match gate_type {
        "H" => gate_matrix_h(),
        "X" => gate_matrix_x(),
        "Y" => gate_matrix_y(),
        "Z" => gate_matrix_z(),
        "I" => gate_matrix_i(),
        "S" => {
            if is_dagger {
                gate_matrix_sdg()
            } else {
                gate_matrix_s()
            }
        }
        "SDG" | "Sdg" => gate_matrix_sdg(),
        "T" => {
            if is_dagger {
                gate_matrix_tdg()
            } else {
                gate_matrix_t()
            }
        }
        "TDG" | "Tdg" => gate_matrix_tdg(),
        "SX" => gate_matrix_sx(),
        "SY" => gate_matrix_sy(),
        "RX" => gate_matrix_rx(params.first().copied().unwrap_or(0.0)),
        "RY" => gate_matrix_ry(params.first().copied().unwrap_or(0.0)),
        "RZ" | "P" => gate_matrix_rz(params.first().copied().unwrap_or(0.0)),
        "U1" => gate_matrix_u1(params.first().copied().unwrap_or(0.0)),
        "U2" => {
            let phi = params.first().copied().unwrap_or(0.0);
            let lambda = params.get(1).copied().unwrap_or(0.0);
            gate_matrix_u2(phi, lambda)
        }
        "U3" => {
            let theta = params.first().copied().unwrap_or(0.0);
            let phi = params.get(1).copied().unwrap_or(0.0);
            let lambda = params.get(2).copied().unwrap_or(0.0);
            gate_matrix_u3(theta, phi, lambda)
        }
        _ => return None,
    };
    Some(m)
}

/// Lift a single-qubit gate to act on qubit `target` in an `n`-qubit system.
/// Result is a 2^n x 2^n matrix: I ⊗ ... ⊗ U ⊗ ... ⊗ I
fn lift_single_gate(u: &UnitaryMatrix, target: usize, num_qubits: usize) -> UnitaryMatrix {
    let n = 1 << num_qubits;
    let mut result = UnitaryMatrix::identity(n);

    for i in 0..n {
        for j in 0..n {
            // Check if i and j differ only on the target qubit
            let mask = !(1 << target);
            if (i & mask) != (j & mask) {
                result.data[i][j] = zero();
                continue;
            }
            let ti = (i >> target) & 1;
            let tj = (j >> target) & 1;
            result.data[i][j] = u.data[ti][tj];
        }
    }
    result
}

/// Lift a controlled gate (control, target) to the full n-qubit space.
/// For standard controlled gates where control acts on target.
fn lift_controlled_gate(
    u: &UnitaryMatrix,
    control: usize,
    target: usize,
    num_qubits: usize,
) -> UnitaryMatrix {
    let n = 1 << num_qubits;
    let mut result = UnitaryMatrix::identity(n);

    for i in 0..n {
        for j in 0..n {
            let c_bit_i = (i >> control) & 1;
            let c_bit_j = (j >> control) & 1;

            // Control bits must match
            if c_bit_i != c_bit_j {
                result.data[i][j] = zero();
                continue;
            }

            // All bits except control and target must match
            let mask = !((1 << control) | (1 << target));
            if (i & mask) != (j & mask) {
                result.data[i][j] = zero();
                continue;
            }

            if c_bit_i == 0 {
                // Control is 0: identity on target
                result.data[i][j] = if i == j { one() } else { zero() };
            } else {
                // Control is 1: apply U on target
                let ti = (i >> target) & 1;
                let tj = (j >> target) & 1;
                // Check rest of bits match
                if (i & mask) == (j & mask) && c_bit_i == c_bit_j {
                    result.data[i][j] = u.data[ti][tj];
                } else {
                    result.data[i][j] = zero();
                }
            }
        }
    }
    result
}

/// Lift a SWAP gate between q1 and q2 into the full n-qubit space.
fn lift_swap_gate(q1: usize, q2: usize, num_qubits: usize) -> UnitaryMatrix {
    let n = 1 << num_qubits;
    let mut result = vec![vec![zero(); n]; n];

    for i in 0..n {
        // Swap bits q1 and q2
        let b1 = (i >> q1) & 1;
        let b2 = (i >> q2) & 1;
        let mut j = i;
        // Clear bits q1, q2
        j &= !((1 << q1) | (1 << q2));
        // Set swapped
        j |= b2 << q1;
        j |= b1 << q2;
        result[j][i] = one();
    }

    UnitaryMatrix {
        data: result,
        dim: n,
    }
}

/// Lift a Toffoli (CCX) gate with given controls and target into n-qubit space.
fn lift_ccx_gate(controls: &[usize], target: usize, num_qubits: usize) -> UnitaryMatrix {
    let n = 1 << num_qubits;
    let mut result = UnitaryMatrix::identity(n);

    for i in 0..n {
        // Check if all control bits are 1
        let all_controls_set = controls.iter().all(|&c| (i >> c) & 1 == 1);
        if all_controls_set {
            let j = i ^ (1 << target); // flip target bit
            result.data[i][i] = zero();
            result.data[i][j] = one();
        }
    }
    result
}

/// Compute the full unitary matrix for the circuit up to (and including) a given step.
/// Returns None if the circuit is too large (> 6 qubits) to avoid huge matrices.
pub fn compute_circuit_unitary(circuit: &Circuit, up_to_step: isize) -> Option<UnitaryMatrix> {
    if circuit.num_qubits == 0 {
        return Some(UnitaryMatrix::identity(1));
    }

    // Limit to 6 qubits (64x64 matrix) to keep things reasonable
    if circuit.num_qubits > 6 {
        return None;
    }

    let n = 1 << circuit.num_qubits;
    let nq = circuit.num_qubits;

    // Collect gates sorted by step
    let mut gates = circuit.gates.clone();
    gates.sort_by_key(|g| g.step);

    let mut result = UnitaryMatrix::identity(n);

    // Group gates by step and process in order
    let mut step_gates: Vec<Vec<&crate::circuit::Gate>> = Vec::new();
    {
        let mut current_step = i64::MIN;
        for g in &gates {
            if up_to_step >= 0 && g.step > up_to_step {
                continue;
            }
            // Skip non-unitary operations
            if g.type_name == "BARRIER"
                || g.type_name == "MEASURE"
                || g.type_name == "MCX"
                || g.type_name == "RESET"
                || g.is_noise
            {
                continue;
            }
            if g.classical_control >= 0 {
                continue;
            }

            if g.step as i64 != current_step {
                current_step = g.step as i64;
                step_gates.push(Vec::new());
            }
            step_gates.last_mut().unwrap().push(g);
        }
    }

    for step_group in &step_gates {
        for gate in step_group {
            let gate_matrix = build_gate_full_matrix(gate, nq);
            if let Some(gm) = gate_matrix {
                result = gm.mul(&result);
            }
        }
    }

    Some(result)
}

/// Build the full n-qubit matrix for a single gate.
fn build_gate_full_matrix(gate: &crate::circuit::Gate, num_qubits: usize) -> Option<UnitaryMatrix> {
    let gate_type = gate.type_name.as_str();

    match gate_type {
        // Two-qubit gates with explicit control
        "CX" => {
            if gate.control >= 0 {
                let u = gate_matrix_x();
                Some(lift_controlled_gate(
                    &u,
                    gate.control as usize,
                    gate.target,
                    num_qubits,
                ))
            } else {
                None
            }
        }
        "CZ" => {
            if gate.control >= 0 {
                let u = gate_matrix_z();
                Some(lift_controlled_gate(
                    &u,
                    gate.control as usize,
                    gate.target,
                    num_qubits,
                ))
            } else {
                None
            }
        }
        "CH" => {
            if gate.control >= 0 {
                let u = gate_matrix_h();
                Some(lift_controlled_gate(
                    &u,
                    gate.control as usize,
                    gate.target,
                    num_qubits,
                ))
            } else {
                None
            }
        }
        "CRX" => {
            if gate.control >= 0 {
                let theta = gate.params.first().copied().unwrap_or(0.0);
                let u = gate_matrix_rx(theta);
                Some(lift_controlled_gate(
                    &u,
                    gate.control as usize,
                    gate.target,
                    num_qubits,
                ))
            } else {
                None
            }
        }
        "CRY" => {
            if gate.control >= 0 {
                let theta = gate.params.first().copied().unwrap_or(0.0);
                let u = gate_matrix_ry(theta);
                Some(lift_controlled_gate(
                    &u,
                    gate.control as usize,
                    gate.target,
                    num_qubits,
                ))
            } else {
                None
            }
        }
        "CRZ" => {
            if gate.control >= 0 {
                let theta = gate.params.first().copied().unwrap_or(0.0);
                let u = gate_matrix_rz(theta);
                Some(lift_controlled_gate(
                    &u,
                    gate.control as usize,
                    gate.target,
                    num_qubits,
                ))
            } else {
                None
            }
        }
        "CU1" => {
            if gate.control >= 0 {
                let lambda = gate.params.first().copied().unwrap_or(0.0);
                let u = gate_matrix_u1(lambda);
                Some(lift_controlled_gate(
                    &u,
                    gate.control as usize,
                    gate.target,
                    num_qubits,
                ))
            } else {
                None
            }
        }
        "SWAP" => {
            if gate.control >= 0 {
                Some(lift_swap_gate(
                    gate.control as usize,
                    gate.target,
                    num_qubits,
                ))
            } else {
                None
            }
        }
        "CCX" => {
            if !gate.controls.is_empty() {
                Some(lift_ccx_gate(&gate.controls, gate.target, num_qubits))
            } else if gate.control >= 0 {
                Some(lift_ccx_gate(
                    &[gate.control as usize],
                    gate.target,
                    num_qubits,
                ))
            } else {
                None
            }
        }
        // Single-qubit gates
        _ => single_qubit_matrix(gate_type, &gate.params, gate.is_dagger)
            .map(|u| lift_single_gate(&u, gate.target, num_qubits)),
    }
}

/// Format a complex number for display.
pub fn format_complex(c: C64) -> String {
    let re = c.re;
    let im = c.im;
    let tol = 1e-10;

    let re_zero = re.abs() < tol;
    let im_zero = im.abs() < tol;

    if re_zero && im_zero {
        return "0".to_string();
    }

    // Check for common exact values
    let re_display = format_component(re);
    let im_display = format_component(im.abs());

    if im_zero {
        return re_display;
    }
    if re_zero {
        let sign = if im < 0.0 { "-" } else { "" };
        if (im.abs() - 1.0).abs() < tol {
            return format!("{}i", sign);
        }
        return format!("{}{}i", sign, im_display);
    }

    let sign = if im < 0.0 { "-" } else { "+" };
    if (im.abs() - 1.0).abs() < tol {
        return format!("{}{}i", re_display, sign);
    }
    format!("{}{}{}i", re_display, sign, im_display)
}

fn format_component(v: f64) -> String {
    let tol = 1e-10;
    let av = v.abs();

    // Check exact values
    if av < tol {
        return "0".to_string();
    }
    if (av - 1.0).abs() < tol {
        return if v < 0.0 {
            "-1".to_string()
        } else {
            "1".to_string()
        };
    }

    // Check 1/sqrt(2)
    let inv_sqrt2 = 1.0 / SQRT_2;
    if (av - inv_sqrt2).abs() < tol {
        return if v < 0.0 {
            "-1/√2".to_string()
        } else {
            "1/√2".to_string()
        };
    }

    // Check 0.5
    if (av - 0.5).abs() < tol {
        return if v < 0.0 {
            "-1/2".to_string()
        } else {
            "1/2".to_string()
        };
    }

    // General case: compact decimal
    if v < 0.0 {
        format!("-{:.3}", av)
    } else {
        format!("{:.3}", av)
    }
}
