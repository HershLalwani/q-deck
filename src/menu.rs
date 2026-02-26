#[derive(Clone, Debug)]
pub struct ParameterHint {
    pub required: bool,
    pub example: &'static str,
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub name: &'static str,
    pub gate_type: &'static str,
    pub symbol: &'static str,
    pub needs_target: bool,
    pub needs_params: bool,
    pub param_hint: Option<ParameterHint>,
}

#[derive(Clone, Debug)]
pub struct MenuCategory {
    pub name: &'static str,
    pub items: &'static [MenuItem],
}

pub static GATE_MENU: &[MenuCategory] = &[
    MenuCategory {
        name: "Single Qubit",
        items: &[
            MenuItem { name: "Hadamard",          gate_type: "H",   symbol: "H",   needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "Pauli-X (NOT)",      gate_type: "X",   symbol: "X",   needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "Pauli-Y",            gate_type: "Y",   symbol: "Y",   needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "Pauli-Z",            gate_type: "Z",   symbol: "Z",   needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "Identity",           gate_type: "I",   symbol: "I",   needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "Phase (S)",          gate_type: "S",   symbol: "S",   needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "Phase Dagger (S†)",  gate_type: "SDG", symbol: "S†",  needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "T Gate",             gate_type: "T",   symbol: "T",   needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "T Dagger (T†)",      gate_type: "TDG", symbol: "T†",  needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "√X (SX)",            gate_type: "SX",  symbol: "√X",  needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "√Y (SY)",            gate_type: "SY",  symbol: "√Y",  needs_target: false, needs_params: false, param_hint: None },
        ],
    },
    MenuCategory {
        name: "Rotation",
        items: &[
            MenuItem { name: "Rotate X",    gate_type: "RX", symbol: "RX", needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "pi/2" }) },
            MenuItem { name: "Rotate Y",    gate_type: "RY", symbol: "RY", needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "pi/2" }) },
            MenuItem { name: "Rotate Z",    gate_type: "RZ", symbol: "RZ", needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "pi/2" }) },
            MenuItem { name: "Phase Shift", gate_type: "P",  symbol: "P",  needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "pi/4" }) },
            MenuItem { name: "Universal U1",gate_type: "U1", symbol: "U1", needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "lambda" }) },
            MenuItem { name: "Universal U2",gate_type: "U2", symbol: "U2", needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "phi,lambda" }) },
            MenuItem { name: "Universal U3",gate_type: "U3", symbol: "U3", needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "theta,phi,lambda" }) },
        ],
    },
    MenuCategory {
        name: "Multi Qubit",
        items: &[
            MenuItem { name: "CNOT",         gate_type: "CX",   symbol: "●─⊕", needs_target: true, needs_params: false, param_hint: None },
            MenuItem { name: "Controlled-Z", gate_type: "CZ",   symbol: "●─●", needs_target: true, needs_params: false, param_hint: None },
            MenuItem { name: "Controlled-H", gate_type: "CH",   symbol: "●─H", needs_target: true, needs_params: false, param_hint: None },
            MenuItem { name: "SWAP",         gate_type: "SWAP", symbol: "×─×", needs_target: true, needs_params: false, param_hint: None },
            MenuItem { name: "Toffoli (CCX)",gate_type: "CCX",  symbol: "●─●─⊕", needs_target: true, needs_params: false, param_hint: None },
            MenuItem { name: "C-Rotate X",  gate_type: "CRX",  symbol: "●─RX", needs_target: true, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "pi/2" }) },
            MenuItem { name: "C-Rotate Y",  gate_type: "CRY",  symbol: "●─RY", needs_target: true, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "pi/2" }) },
            MenuItem { name: "C-Rotate Z",  gate_type: "CRZ",  symbol: "●─RZ", needs_target: true, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "pi/2" }) },
            MenuItem { name: "C-Phase (CU1)",gate_type: "CU1",  symbol: "●─U1", needs_target: true, needs_params: true, param_hint: Some(ParameterHint { required: true, example: "lambda" }) },
        ],
    },
    MenuCategory {
        name: "Measurement",
        items: &[
            MenuItem { name: "Measure",         gate_type: "MEASURE", symbol: "M",   needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "Measure-Ctrl X",  gate_type: "MCX",     symbol: "M─⊕", needs_target: true,  needs_params: false, param_hint: None },
        ],
    },
    MenuCategory {
        name: "Special",
        items: &[
            MenuItem { name: "Reset",   gate_type: "RESET",   symbol: "|0⟩", needs_target: false, needs_params: false, param_hint: None },
            MenuItem { name: "Barrier", gate_type: "BARRIER", symbol: "┃",   needs_target: false, needs_params: false, param_hint: None },
        ],
    },
    MenuCategory {
        name: "Noise",
        items: &[
            MenuItem { name: "Depolarizing",      gate_type: "NOISE_DEPOL",  symbol: "N", needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: false, example: "0.01" }) },
            MenuItem { name: "Amplitude Damping", gate_type: "NOISE_AMP",   symbol: "N", needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: false, example: "0.01" }) },
            MenuItem { name: "Phase Damping",     gate_type: "NOISE_PHASE", symbol: "N", needs_target: false, needs_params: true, param_hint: Some(ParameterHint { required: false, example: "0.01" }) },
        ],
    },
];

pub fn is_parameterized_gate(gate_type: &str) -> bool {
    matches!(
        gate_type,
        "RX" | "RY" | "RZ" | "P" | "U1" | "U2" | "U3"
            | "CRX" | "CRY" | "CRZ" | "CU1"
            | "NOISE_DEPOL" | "NOISE_AMP" | "NOISE_PHASE"
    )
}
