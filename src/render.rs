use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus};
use crate::circuit::{CellInfo, Circuit};
use crate::matrix::{compute_circuit_unitary, format_complex};
use crate::menu::GATE_MENU;
use crate::quantum::simulate_circuit;

// ── Colors ─────────────────────────────────────────────────────────────────

const BLUE: Color = Color::Rgb(122, 162, 247);
const PURPLE: Color = Color::Rgb(187, 154, 247);
const GREEN: Color = Color::Rgb(158, 206, 106);
const ORANGE: Color = Color::Rgb(255, 158, 100);
const CYAN: Color = Color::Rgb(115, 218, 202);
const YELLOW: Color = Color::Rgb(224, 175, 104);
const DIM: Color = Color::Rgb(86, 95, 137);
const RED: Color = Color::Rgb(247, 118, 142);
const DARK_BLUE: Color = Color::Rgb(192, 202, 245);

// ── Layout constants ────────────────────────────────────────────────────────

const CELL_W: usize = 11;
const LABEL_W: usize = 7; // "q[N]  ──"
const GATE_NAME_W: usize = 5;

// ── Main render entry point ─────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();
    app.width = size.width;
    app.height = size.height;

    let ctrl_height = 3u16;
    let avail_h = size.height.saturating_sub(ctrl_height);

    // Left/Right split
    let qasm_w = ((size.width / 3) as usize)
        .max(30)
        .min((size.width - 20) as usize) as u16;
    let left_w = size.width.saturating_sub(qasm_w);

    let state_h = if avail_h < 20 { avail_h / 3 } else { 13 }
        .max(4)
        .min(avail_h.saturating_sub(3));
    let circuit_h = avail_h.saturating_sub(state_h).max(1);

    // Main layout: [top_row, controls]
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(avail_h), Constraint::Length(ctrl_height)])
        .split(size);

    // Top row: [left_col, qasm]
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(left_w), Constraint::Min(qasm_w)])
        .split(main_chunks[0]);

    // Left column: [circuit, state]
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(circuit_h), Constraint::Min(state_h)])
        .split(top_chunks[0]);

    render_circuit_panel(f, app, left_chunks[0]);
    if app.show_matrix {
        render_matrix_panel(f, app, left_chunks[1]);
    } else {
        render_state_panel(f, app, left_chunks[1]);
    }
    render_qasm_panel(f, app, top_chunks[1]);
    render_controls_panel(f, app, main_chunks[1]);

    // Overlays
    match app.focus {
        Focus::Menu => render_menu_overlay(f, app),
        Focus::InputParam | Focus::EditParam => render_param_input_overlay(f, app),
        Focus::EditGate => render_edit_gate_overlay(f, app),
        _ => {}
    }
}

// ── Circuit Panel ─────────────────────────────────────────────────────────────

fn render_circuit_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let active = matches!(
        app.focus,
        Focus::Circuit
            | Focus::SelectTarget
            | Focus::Menu
            | Focus::SelectControls
            | Focus::EditGate
            | Focus::EditTarget
            | Focus::EditControl
    );
    let border_color = if active { ORANGE } else { BLUE };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            "Quantum Circuit",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let circuit = app.circuit();
    let lines = build_circuit_lines(app, &circuit, inner.width as usize, inner.height as usize);

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

fn build_circuit_lines(
    app: &mut App,
    circuit: &Circuit,
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let wire_style = Style::default().fg(Color::White);

    // Header line
    let avail_w = width.saturating_sub(LABEL_W + 2);
    let max_steps = (avail_w / CELL_W).max(1);

    let start_step = if app.cursor_step >= max_steps as isize {
        (app.cursor_step - max_steps as isize + 1) as usize
    } else {
        0
    };

    // Step numbers header
    let mut step_hdr_spans = vec![Span::styled(" ".repeat(LABEL_W), wire_style)];
    for step in start_step..start_step + max_steps {
        step_hdr_spans.push(Span::styled(
            pad_center(&format!("{step}"), CELL_W),
            wire_style,
        ));
    }
    lines.push(Line::from(step_hdr_spans));

    // Qubit rows (3 lines each)
    let num_cbits = circuit.num_cbits();
    let cbit_lines = if num_cbits > 0 { 2 } else { 0 };
    let status_lines = 1;
    let header_lines = 1;
    let avail_h = height.saturating_sub(header_lines + cbit_lines + status_lines);
    let max_qubits = (avail_h / 3).max(1);

    // Track which qubit is "active" for scrolling purposes
    let active_qubit = if matches!(
        app.focus,
        Focus::SelectTarget | Focus::SelectControls | Focus::EditTarget | Focus::EditControl
    ) {
        app.target_qubit
    } else {
        app.cursor_qubit
    };

    // Keep active qubit in view
    if active_qubit >= app.qubit_scroll + max_qubits {
        app.qubit_scroll = active_qubit + 1 - max_qubits;
    } else if active_qubit < app.qubit_scroll {
        app.qubit_scroll = active_qubit;
    }

    let start_qubit = app.qubit_scroll;
    let end_qubit = (start_qubit + max_qubits).min(circuit.num_qubits);

    for qubit in start_qubit..end_qubit {
        let mut top_line_spans = vec![Span::raw(" ".repeat(LABEL_W))];
        let label = format!("q[{qubit}]");
        let mut mid_line_spans = vec![
            Span::styled(format!("{:<5}", label), wire_style),
            Span::styled("──", wire_style),
        ];
        let mut bot_line_spans = vec![Span::raw(" ".repeat(LABEL_W))];

        for step_idx in start_step..start_step + max_steps {
            let step = step_idx as isize;
            let info = circuit.get_cell_info(step, qubit);

            let is_cursor = step == app.cursor_step
                && qubit == app.cursor_qubit
                && matches!(
                    app.focus,
                    Focus::Circuit
                        | Focus::SelectTarget
                        | Focus::Menu
                        | Focus::SelectControls
                        | Focus::EditGate
                );

            let current_step = if matches!(app.focus, Focus::EditTarget | Focus::EditControl) {
                app.edit_orig_step
            } else {
                app.cursor_step
            };
            let is_target_sel = step == current_step
                && qubit == app.target_qubit
                && matches!(
                    app.focus,
                    Focus::SelectTarget
                        | Focus::SelectControls
                        | Focus::EditTarget
                        | Focus::EditControl
                );

            let (top, mid, bot) = render_cell(&info, is_cursor, is_target_sel, qubit);
            top_line_spans.extend(top);
            mid_line_spans.extend(mid);
            bot_line_spans.extend(bot);
        }

        lines.push(Line::from(top_line_spans));
        lines.push(Line::from(mid_line_spans));
        lines.push(Line::from(bot_line_spans));
    }

    // Classical bit wire
    let num_cbits = circuit.num_cbits();
    if num_cbits > 0 {
        let mut sep_spans = vec![Span::raw(" ".repeat(LABEL_W))];
        for step_idx in start_step..start_step + max_steps {
            let mq = circuit.get_measure_at_step(step_idx as isize);
            if mq >= 0 {
                let half = CELL_W / 2;
                sep_spans.push(Span::styled(" ".repeat(half), wire_style));
                sep_spans.push(Span::styled("║", wire_style));
                sep_spans.push(Span::styled(" ".repeat(CELL_W - half - 1), wire_style));
            } else {
                sep_spans.push(Span::styled(" ".repeat(CELL_W), wire_style));
            }
        }
        lines.push(Line::from(sep_spans));

        let cbit_label = format!("c{num_cbits}");
        let mut cbit_line_spans = vec![
            Span::styled(format!("{:<5}", cbit_label), wire_style),
            Span::styled("══", wire_style),
        ];
        for step_idx in start_step..start_step + max_steps {
            let mq = circuit.get_measure_at_step(step_idx as isize);
            if mq >= 0 {
                let bit_label = format!("{mq}");
                let dash_l = (CELL_W - 1) / 2;
                let dash_r = CELL_W.saturating_sub(dash_l + 1 + bit_label.len());
                cbit_line_spans.push(Span::styled("═".repeat(dash_l), wire_style));
                cbit_line_spans.push(Span::styled(format!("╩{bit_label}"), wire_style));
                cbit_line_spans.push(Span::styled("═".repeat(dash_r), wire_style));
            } else {
                cbit_line_spans.push(Span::styled("═".repeat(CELL_W), wire_style));
            }
        }
        lines.push(Line::from(cbit_line_spans));
    }

    // Status / position line
    let more_above = start_qubit > 0;
    let more_below = end_qubit < circuit.num_qubits;
    let scroll_msg = if more_above && more_below {
        "  (↑↓ More qubits)"
    } else if more_above {
        "  (↑ More qubits)"
    } else if more_below {
        "  (↓ More qubits)"
    } else {
        ""
    };

    match app.focus {
        Focus::SelectTarget => {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "  {} Select target: q[{}]",
                        app.pending_gate, app.target_qubit
                    ),
                    Style::default().fg(YELLOW),
                ),
                Span::styled(
                    format!("  ↑↓ Move  Enter Confirm  Esc Cancel{}", scroll_msg),
                    Style::default().fg(DIM),
                ),
            ]));
        }
        Focus::SelectControls => {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "  {} Select control: q[{}]",
                        app.pending_gate, app.target_qubit
                    ),
                    Style::default().fg(YELLOW),
                ),
                Span::styled(
                    format!("  ↑↓ Move  Enter Next  Esc Cancel{}", scroll_msg),
                    Style::default().fg(DIM),
                ),
            ]));
        }
        Focus::EditTarget => {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  Edit target: q[{}]", app.target_qubit),
                    Style::default().fg(YELLOW),
                ),
                Span::styled(
                    format!("  ↑↓ Move  Enter Confirm  Esc Cancel{}", scroll_msg),
                    Style::default().fg(DIM),
                ),
            ]));
        }
        Focus::EditControl => {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  Edit control: q[{}]", app.target_qubit),
                    Style::default().fg(YELLOW),
                ),
                Span::styled(
                    format!("  ↑↓ Move  Enter Confirm  Esc Cancel{}", scroll_msg),
                    Style::default().fg(DIM),
                ),
            ]));
        }
        _ => {
            let mut status_spans = vec![
                Span::styled(
                    format!(
                        "  Position: Step {}, Qubit {}",
                        app.cursor_step, app.cursor_qubit
                    ),
                    Style::default().fg(DIM),
                ),
                Span::styled(scroll_msg, Style::default().fg(DIM)),
            ];
            if !app.status_msg.is_empty() {
                status_spans.push(Span::styled(
                    format!("  │  {}", app.status_msg),
                    Style::default().fg(YELLOW),
                ));
            }
            lines.push(Line::from(status_spans));
        }
    }

    lines
}

fn render_cell(
    info: &CellInfo,
    is_cursor: bool,
    is_target_sel: bool,
    qubit: usize,
) -> (Vec<Span<'static>>, Vec<Span<'static>>, Vec<Span<'static>>) {
    let half = CELL_W / 2;
    let dash_l_len = (CELL_W - 1) / 2;
    let dash_r_len = CELL_W - dash_l_len - 1;

    let wire_style = Style::default().fg(Color::White);
    let gate_style = Style::default().fg(BLUE);
    let measure_style = Style::default().fg(YELLOW);

    let vert_row = vec![
        Span::styled(" ".repeat(half), wire_style),
        Span::styled("│", wire_style),
        Span::styled(" ".repeat(CELL_W - half - 1), wire_style),
    ];
    let dbl_vert_row = vec![
        Span::styled(" ".repeat(half), wire_style),
        Span::styled("║", wire_style),
        Span::styled(" ".repeat(CELL_W - half - 1), wire_style),
    ];
    let empty_row = vec![Span::styled(" ".repeat(CELL_W), wire_style)];

    if is_cursor || is_target_sel {
        let sel_color = if is_cursor { ORANGE } else { CYAN };
        let sel_style = Style::default().fg(sel_color);
        let inner_w = CELL_W - 2;
        let dleft = (inner_w - 1) / 2;
        let dright = inner_w - dleft - 1;

        if info.is_barrier {
            let mid = vec![
                Span::styled("║", sel_style),
                Span::styled("─".repeat(dleft), wire_style),
                Span::styled("│", wire_style),
                Span::styled("─".repeat(dright), wire_style),
                Span::styled("║", sel_style),
            ];
            return (vert_row.clone(), mid, vert_row.clone());
        }

        let top = vec![
            Span::styled("╔", sel_style),
            Span::styled("═".repeat(inner_w), sel_style),
            Span::styled("╗", sel_style),
        ];
        let bot = vec![
            Span::styled("╚", sel_style),
            Span::styled("═".repeat(inner_w), sel_style),
            Span::styled("╝", sel_style),
        ];

        let mut mid = vec![Span::styled("║", sel_style)];
        if let Some(gate) = &info.gate {
            if info.is_control {
                let sym = control_symbol(&gate.type_name);
                mid.push(Span::styled("─".repeat(dleft), wire_style));
                mid.push(Span::styled(sym, gate_style));
                mid.push(Span::styled("─".repeat(dright), wire_style));
            } else if info.is_target && is_symbol_gate(&gate.type_name) {
                let sym = target_symbol(&gate.type_name);
                mid.push(Span::styled("─".repeat(dleft), wire_style));
                mid.push(Span::styled(sym, gate_style));
                mid.push(Span::styled("─".repeat(dright), wire_style));
            } else if info.is_target
                || (gate.measure_source < 0
                    && gate.type_name != "MEASURE"
                    && gate.type_name != "BARRIER")
            {
                let name = pad_center(&gate_display_name(&gate.type_name), GATE_NAME_W);
                mid.push(Span::styled("─", wire_style));
                mid.push(Span::styled("┤", gate_style));
                mid.push(Span::styled(name, gate_style));
                mid.push(Span::styled("├", gate_style));
                mid.push(Span::styled("─", wire_style));
            } else if gate.measure_source >= 0 {
                let is_m = gate.measure_source as usize == qubit;
                let sym = if is_m { "M" } else { "⊕" };
                let style = if is_m { measure_style } else { gate_style };
                mid.push(Span::styled("─".repeat(dleft), wire_style));
                mid.push(Span::styled(sym, style));
                mid.push(Span::styled("─".repeat(dright), wire_style));
            } else {
                mid.push(Span::styled("─".repeat(inner_w), wire_style));
            }
        } else if info.pass_through {
            mid.push(Span::styled("─".repeat(dleft), wire_style));
            mid.push(Span::styled("┼", wire_style));
            mid.push(Span::styled("─".repeat(dright), wire_style));
        } else {
            mid.push(Span::styled("─".repeat(inner_w), wire_style));
        }
        mid.push(Span::styled("║", sel_style));

        return (top, mid, bot);
    }

    // Normal cells
    if info.is_barrier {
        let mid = vec![
            Span::styled("─".repeat(dash_l_len), wire_style),
            Span::styled("│", wire_style),
            Span::styled("─".repeat(dash_r_len), wire_style),
        ];
        return (vert_row.clone(), mid, vert_row.clone());
    }

    if let Some(gate) = &info.gate {
        if info.is_control {
            let top = if info.vert_above {
                vert_row.clone()
            } else {
                empty_row.clone()
            };
            let sym = control_symbol(&gate.type_name);
            let mid = vec![
                Span::styled("─".repeat(dash_l_len), wire_style),
                Span::styled(sym, gate_style),
                Span::styled("─".repeat(dash_r_len), wire_style),
            ];
            let bot = if info.measure_below {
                dbl_vert_row.clone()
            } else if info.vert_below {
                vert_row.clone()
            } else {
                empty_row.clone()
            };
            return (top, mid, bot);
        }
        if info.is_target {
            if is_symbol_gate(&gate.type_name) {
                let top = if info.vert_above {
                    vert_row.clone()
                } else {
                    empty_row.clone()
                };
                let sym = target_symbol(&gate.type_name);
                let mid = vec![
                    Span::styled("─".repeat(dash_l_len), wire_style),
                    Span::styled(sym, gate_style),
                    Span::styled("─".repeat(dash_r_len), wire_style),
                ];
                let bot = if info.measure_below {
                    dbl_vert_row.clone()
                } else if info.vert_below {
                    vert_row.clone()
                } else {
                    empty_row.clone()
                };
                return (top, mid, bot);
            } else {
                // Controlled gate box
                let margin = (CELL_W - GATE_NAME_W - 2) / 2;
                let rmargin = CELL_W - margin - GATE_NAME_W - 2;
                let name = pad_center(&gate_display_name(&gate.type_name), GATE_NAME_W);
                let top = vec![
                    Span::styled(" ".repeat(margin), wire_style),
                    Span::styled(if info.vert_above { "┬" } else { "┌" }, gate_style),
                    Span::styled("─".repeat(GATE_NAME_W), gate_style),
                    Span::styled(if info.vert_above { "┬" } else { "┐" }, gate_style),
                    Span::styled(" ".repeat(rmargin), wire_style),
                ];
                let mid = vec![
                    Span::styled("─".repeat(margin), wire_style),
                    Span::styled("┤", gate_style),
                    Span::styled(name, gate_style),
                    Span::styled("├", gate_style),
                    Span::styled("─".repeat(rmargin), wire_style),
                ];
                let bot = if info.measure_below {
                    dbl_vert_row.clone()
                } else {
                    vec![
                        Span::styled(" ".repeat(margin), wire_style),
                        Span::styled(if info.vert_below { "┴" } else { "└" }, gate_style),
                        Span::styled("─".repeat(GATE_NAME_W), gate_style),
                        Span::styled(if info.vert_below { "┴" } else { "┘" }, gate_style),
                        Span::styled(" ".repeat(rmargin), wire_style),
                    ]
                };
                return (top, mid, bot);
            }
        }
        if gate.measure_source >= 0 {
            let margin = (CELL_W - GATE_NAME_W - 2) / 2;
            let rmargin = CELL_W - margin - GATE_NAME_W - 2;
            if gate.measure_source as usize == qubit {
                let top = vec![
                    Span::styled(" ".repeat(margin), wire_style),
                    Span::styled("┌", measure_style),
                    Span::styled("─".repeat(GATE_NAME_W), measure_style),
                    Span::styled("┐", measure_style),
                    Span::styled(" ".repeat(rmargin), wire_style),
                ];
                let mid = vec![
                    Span::styled("─".repeat(margin), wire_style),
                    Span::styled("┤", measure_style),
                    Span::styled(pad_center("M", GATE_NAME_W), measure_style),
                    Span::styled("├", measure_style),
                    Span::styled("─".repeat(rmargin), wire_style),
                ];
                let bot = if info.measure_below {
                    dbl_vert_row.clone()
                } else {
                    vec![
                        Span::styled(" ".repeat(margin), wire_style),
                        Span::styled("└", measure_style),
                        Span::styled("─".repeat(GATE_NAME_W), measure_style),
                        Span::styled("┘", measure_style),
                        Span::styled(" ".repeat(rmargin), wire_style),
                    ]
                };
                return (top, mid, bot);
            } else if gate.target == qubit {
                let top = if info.vert_above {
                    vert_row.clone()
                } else {
                    empty_row.clone()
                };
                let mid = vec![
                    Span::styled("─".repeat(dash_l_len), wire_style),
                    Span::styled("⊕", gate_style),
                    Span::styled("─".repeat(dash_r_len), wire_style),
                ];
                let bot = if info.measure_below {
                    dbl_vert_row.clone()
                } else if info.vert_below {
                    vert_row.clone()
                } else {
                    empty_row.clone()
                };
                return (top, mid, bot);
            }
        }
        if gate.type_name == "MEASURE" {
            let margin = (CELL_W - GATE_NAME_W - 2) / 2;
            let rmargin = CELL_W - margin - GATE_NAME_W - 2;
            let top = vec![
                Span::styled(" ".repeat(margin), wire_style),
                Span::styled("┌", measure_style),
                Span::styled("─".repeat(GATE_NAME_W), measure_style),
                Span::styled("┐", measure_style),
                Span::styled(" ".repeat(rmargin), wire_style),
            ];
            let mid = vec![
                Span::styled("─".repeat(margin), wire_style),
                Span::styled("┤", measure_style),
                Span::styled(pad_center("M", GATE_NAME_W), measure_style),
                Span::styled("├", measure_style),
                Span::styled("─".repeat(rmargin), wire_style),
            ];
            let bot = vec![
                Span::styled(" ".repeat(margin), wire_style),
                Span::styled("└", measure_style),
                Span::styled("─".repeat(GATE_NAME_W), measure_style),
                Span::styled("┘", measure_style),
                Span::styled(" ".repeat(rmargin), wire_style),
            ];
            return (top, mid, bot);
        }
        // Normal single-qubit gate box
        let margin = (CELL_W - GATE_NAME_W - 2) / 2;
        let rmargin = CELL_W - margin - GATE_NAME_W - 2;
        let name = pad_center(&gate_display_name(&gate.type_name), GATE_NAME_W);
        let top = vec![
            Span::styled(" ".repeat(margin), wire_style),
            Span::styled("┌", gate_style),
            Span::styled("─".repeat(GATE_NAME_W), gate_style),
            Span::styled("┐", gate_style),
            Span::styled(" ".repeat(rmargin), wire_style),
        ];
        let mid = vec![
            Span::styled("─".repeat(margin), wire_style),
            Span::styled("┤", gate_style),
            Span::styled(name, gate_style),
            Span::styled("├", gate_style),
            Span::styled("─".repeat(rmargin), wire_style),
        ];
        let bot = if info.measure_below {
            dbl_vert_row.clone()
        } else {
            vec![
                Span::styled(" ".repeat(margin), wire_style),
                Span::styled("└", gate_style),
                Span::styled("─".repeat(GATE_NAME_W), gate_style),
                Span::styled("┘", gate_style),
                Span::styled(" ".repeat(rmargin), wire_style),
            ]
        };
        return (top, mid, bot);
    }

    if info.pass_through {
        let mid = vec![
            Span::styled("─".repeat(dash_l_len), wire_style),
            Span::styled("┼", wire_style),
            Span::styled("─".repeat(dash_r_len), wire_style),
        ];
        let bot = if info.measure_below {
            dbl_vert_row.clone()
        } else {
            vert_row.clone()
        };
        return (vert_row.clone(), mid, bot);
    }

    if info.measure_below {
        let top = if info.vert_above {
            vert_row.clone()
        } else {
            dbl_vert_row.clone()
        };
        let mid = vec![
            Span::styled("─".repeat(dash_l_len), wire_style),
            Span::styled("╫", wire_style),
            Span::styled("─".repeat(dash_r_len), wire_style),
        ];
        return (top, mid, dbl_vert_row.clone());
    }

    // Empty wire
    let top = if info.vert_above {
        vert_row.clone()
    } else {
        empty_row.clone()
    };
    let mid = vec![Span::styled("─".repeat(CELL_W), wire_style)];
    let bot = if info.vert_below {
        vert_row.clone()
    } else {
        empty_row.clone()
    };
    (top, mid, bot)
}

fn control_symbol(gate_type: &str) -> String {
    if gate_type == "SWAP" {
        "×".to_string()
    } else {
        "●".to_string()
    }
}

fn is_symbol_gate(gate_type: &str) -> bool {
    matches!(gate_type, "CX" | "CCX" | "MCX" | "SWAP")
}

fn target_symbol(gate_type: &str) -> String {
    match gate_type {
        "CZ" => "●".to_string(),
        "SWAP" => "×".to_string(),
        "CX" | "CCX" | "MCX" => "⊕".to_string(),
        _ => "⊕".to_string(),
    }
}

fn gate_display_name(gate_type: &str) -> String {
    match gate_type {
        "MEASURE" => "M".to_string(),
        "CX" | "CCX" | "MCX" => "X".to_string(),
        "CZ" => "Z".to_string(),
        "CH" => "H".to_string(),
        "CU1" | "CP" => "U1".to_string(),
        "CRX" => "RX".to_string(),
        "CRY" => "RY".to_string(),
        "CRZ" => "RZ".to_string(),
        other => {
            if other.starts_with('C') && other.len() > 1 && other != "CONTROL" {
                other[1..].to_string()
            } else {
                other.to_string()
            }
        }
    }
}

fn pad_center(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.chars().take(width).collect();
    }
    let total = width - len;
    let left = total / 2;
    let right = total - left;
    " ".repeat(left) + s + &" ".repeat(right)
}

// ── State / Probabilities Panel ───────────────────────────────────────────────

fn render_state_panel(f: &mut Frame, app: &App, area: Rect) {
    let border_color = { RED };
    let title = if app.show_statevector {
        "Statevector"
    } else {
        "Probabilities"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let circuit = app.circuit();
    let state = simulate_circuit(&circuit, app.cursor_step);
    let mut qsphere = state.get_qsphere_states();
    qsphere.sort_by(|a, b| {
        b.prob
            .partial_cmp(&a.prob)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let num_qubits = circuit.num_qubits.max(app.dag.num_qubits).max(1);
    let mut text_lines: Vec<Line> = Vec::new();

    if app.show_statevector {
        // Statevector view: show complex amplitudes
        let display_count = qsphere.len().min(16);
        for s in qsphere.iter().take(display_count) {
            let state_str = format_basis_state(s.basis_state, num_qubits);
            let re = s.amplitude.re;
            let im = s.amplitude.im;
            let sign = if im >= 0.0 { '+' } else { '-' };
            let line_str = format!(
                "{}  α={:+.4}{}{:.4}i  P={:.4}  φ={:.4}",
                state_str,
                re,
                sign,
                im.abs(),
                s.prob,
                s.phase
            );
            text_lines.push(Line::styled(line_str, Style::default().fg(CYAN)));
        }

        if qsphere.len() > 16 {
            text_lines.push(Line::styled(
                format!("... and {} more states", qsphere.len() - 16),
                Style::default().fg(DIM),
            ));
        }

        // Footer
        if let Some(top) = qsphere.first() {
            text_lines.push(Line::default());
            text_lines.push(Line::styled(
                format!(
                    "Top: {} ({:.1}%)  {} non-zero",
                    format_basis_state(top.basis_state, num_qubits),
                    top.prob * 100.0,
                    qsphere.len()
                ),
                Style::default().fg(DIM),
            ));
        }
    } else {
        // Probabilities view: show bar chart
        let bar_width = (inner.width as usize).saturating_sub(30).max(10);

        let display_count = qsphere.len().min(16);
        for s in qsphere.iter().take(display_count) {
            let fill = ((s.prob * bar_width as f64).round() as usize).min(bar_width);
            let empty = bar_width - fill;
            let bar = "█".repeat(fill) + &"░".repeat(empty);
            let state_str = format_basis_state(s.basis_state, num_qubits);
            let line_str = format!("{}: P={:.2} [{}]", state_str, s.prob, bar);
            text_lines.push(Line::styled(line_str, Style::default().fg(YELLOW)));
        }

        if qsphere.len() > 16 {
            text_lines.push(Line::styled(
                format!("... and {} more states", qsphere.len() - 16),
                Style::default().fg(DIM),
            ));
        }

        // Footer
        if let Some(top) = qsphere.first() {
            text_lines.push(Line::default());
            text_lines.push(Line::styled(
                format!(
                    "Top: {} ({:.1}%)  {} non-zero",
                    format_basis_state(top.basis_state, num_qubits),
                    top.prob * 100.0,
                    qsphere.len()
                ),
                Style::default().fg(DIM),
            ));
        }
    }

    let p = Paragraph::new(Text::from(text_lines)).wrap(Wrap { trim: false });
    f.render_widget(p, inner);
}

fn format_basis_state(state: usize, num_qubits: usize) -> String {
    let mut s = String::from("|");
    for i in (0..num_qubits).rev() {
        s.push(if state & (1 << i) != 0 { '1' } else { '0' });
    }
    s.push('⟩');
    s
}

// ── Matrix Panel ──────────────────────────────────────────────────────────────

fn render_matrix_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let border_color = RED;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            "Circuit Matrix (Unitary)",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let circuit = app.circuit();
    let num_qubits = circuit.num_qubits.max(app.dag.num_qubits).max(1);

    if num_qubits > 6 {
        let lines = vec![
            Line::default(),
            Line::styled(
                "  Matrix view limited to 6 qubits (64x64)",
                Style::default().fg(YELLOW),
            ),
            Line::styled(
                format!("  Current circuit has {} qubits", num_qubits),
                Style::default().fg(DIM),
            ),
            Line::default(),
            Line::styled(
                "  Press m to return to state view",
                Style::default().fg(DIM),
            ),
        ];
        let p = Paragraph::new(Text::from(lines));
        f.render_widget(p, inner);
        return;
    }

    let unitary = compute_circuit_unitary(&circuit, app.cursor_step);

    match unitary {
        None => {
            let lines = vec![
                Line::default(),
                Line::styled(
                    "  Could not compute circuit matrix",
                    Style::default().fg(YELLOW),
                ),
            ];
            let p = Paragraph::new(Text::from(lines));
            f.render_widget(p, inner);
        }
        Some(matrix) => {
            let dim = matrix.dim;
            let inner_h = inner.height as usize;
            let inner_w = inner.width as usize;

            // Build a bit-reversal permutation so that q[0] (top of circuit)
            // maps to the leftmost bit in the ket labels, matching textbook convention.
            let bit_reverse = |idx: usize, nq: usize| -> usize {
                let mut result = 0;
                for b in 0..nq {
                    if (idx >> b) & 1 == 1 {
                        result |= 1 << (nq - 1 - b);
                    }
                }
                result
            };

            // Determine column width based on available space
            let label_w = num_qubits + 3; // "|01⟩" width
            let col_w = if inner_w > 0 {
                // Each column: value + spacing
                let avail = inner_w.saturating_sub(label_w + 2);
                let max_cols = dim;
                let ideal = avail / max_cols;
                ideal.clamp(6, 10)
            } else {
                8
            };

            let visible_cols = ((inner_w.saturating_sub(label_w + 2)) / col_w).min(dim);

            let mut text_lines: Vec<Line> = Vec::new();

            // Header row with column labels
            let mut header_spans: Vec<Span> = vec![Span::styled(
                " ".repeat(label_w + 1),
                Style::default().fg(DIM),
            )];
            for c in 0..visible_cols {
                let col_label = format_basis_ket(c, num_qubits);
                header_spans.push(Span::styled(
                    pad_to_width(&col_label, col_w),
                    Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
                ));
            }
            if visible_cols < dim {
                header_spans.push(Span::styled("...", Style::default().fg(DIM)));
            }
            text_lines.push(Line::from(header_spans));

            // Clamp scroll
            let max_scroll = dim.saturating_sub(inner_h.saturating_sub(3));
            if app.matrix_scroll > max_scroll {
                app.matrix_scroll = max_scroll;
            }

            // Matrix rows
            let visible_rows = (inner_h.saturating_sub(3)).min(dim); // -1 header, -1 footer, -1 buffer
            let start_row = app.matrix_scroll;
            let end_row = (start_row + visible_rows).min(dim);

            for r in start_row..end_row {
                let row_label = format_basis_bra(r, num_qubits);
                let mut row_spans: Vec<Span> = vec![Span::styled(
                    format!("{} ", pad_to_width(&row_label, label_w)),
                    Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
                )];

                // Map display row/col to internal indices via bit-reversal
                let internal_r = bit_reverse(r, num_qubits);
                for c in 0..visible_cols {
                    let internal_c = bit_reverse(c, num_qubits);
                    let val = matrix.data[internal_r][internal_c];
                    let formatted = format_complex(val);
                    let color = if val.norm_sqr() < 1e-20 {
                        DIM
                    } else if (val.im.abs()) < 1e-10 {
                        GREEN
                    } else if (val.re.abs()) < 1e-10 {
                        CYAN
                    } else {
                        DARK_BLUE
                    };
                    row_spans.push(Span::styled(
                        pad_to_width(&formatted, col_w),
                        Style::default().fg(color),
                    ));
                }

                if visible_cols < dim {
                    row_spans.push(Span::styled("...", Style::default().fg(DIM)));
                }

                text_lines.push(Line::from(row_spans));
            }

            // Footer
            if dim > visible_rows || dim > visible_cols {
                text_lines.push(Line::default());
                let footer = format!(
                    "  {}x{} unitary  (showing rows {}-{}, {} cols)  Step {}",
                    dim,
                    dim,
                    start_row,
                    end_row.saturating_sub(1),
                    visible_cols.min(dim),
                    app.cursor_step
                );
                text_lines.push(Line::styled(footer, Style::default().fg(DIM)));
            } else {
                text_lines.push(Line::default());
                text_lines.push(Line::styled(
                    format!("  {}x{} unitary at step {}", dim, dim, app.cursor_step),
                    Style::default().fg(DIM),
                ));
            }

            let p = Paragraph::new(Text::from(text_lines));
            f.render_widget(p, inner);
        }
    }
}

fn format_basis_ket(state: usize, num_qubits: usize) -> String {
    let mut s = String::from("|");
    for i in (0..num_qubits).rev() {
        s.push(if state & (1 << i) != 0 { '1' } else { '0' });
    }
    s.push('⟩');
    s
}

fn format_basis_bra(state: usize, num_qubits: usize) -> String {
    let mut s = String::from("⟨");
    for i in (0..num_qubits).rev() {
        s.push(if state & (1 << i) != 0 { '1' } else { '0' });
    }
    s.push('|');
    s
}

fn pad_to_width(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.chars().take(width).collect()
    } else {
        let mut out = s.to_string();
        for _ in 0..(width - len) {
            out.push(' ');
        }
        out
    }
}

// ── QASM Panel ────────────────────────────────────────────────────────────────

fn render_qasm_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Qasm;
    let border_color = if active { ORANGE } else { PURPLE };
    let mut title = if active {
        "QASM Editor [ACTIVE]"
    } else {
        "QASM Editor"
    }
    .to_string();

    if !app.qasm_errors.is_empty() {
        title = format!("{} ({} ERRORS)", title, app.qasm_errors.len());
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let inner_h = inner.height as usize;

    if active {
        let (cursor_row, cursor_col) = app.qasm_cursor_row_col();

        // Keep cursor in view
        if cursor_row < app.qasm_scroll as usize {
            app.qasm_scroll = cursor_row as u16;
        }
        if inner_h > 0 && cursor_row >= app.qasm_scroll as usize + inner_h {
            app.qasm_scroll = (cursor_row + 1 - inner_h) as u16;
        }
        let scroll = app.qasm_scroll as usize;

        let text_lines: Vec<&str> = app.qasm_text.split('\n').collect();
        let mut lines: Vec<Line> = Vec::new();

        for (i, line_str) in text_lines.iter().enumerate().skip(scroll).take(inner_h) {
            let is_error = app.qasm_errors.iter().any(|(line_idx, _)| *line_idx == i);
            let base_style = if is_error {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(DARK_BLUE)
            };

            if i == cursor_row {
                let safe_col = cursor_col.min(line_str.len());
                let before = &line_str[..safe_col];
                let (cur_ch, after): (&str, &str) = if safe_col < line_str.len() {
                    let ch = line_str[safe_col..].chars().next().unwrap();
                    let end = safe_col + ch.len_utf8();
                    (&line_str[safe_col..end], &line_str[end..])
                } else {
                    (" ", "")
                };
                lines.push(Line::from(vec![
                    Span::styled(before, base_style),
                    Span::styled(cur_ch, Style::default().fg(Color::Black).bg(ORANGE)),
                    Span::styled(after, base_style),
                ]));
            } else {
                lines.push(Line::styled(*line_str, base_style));
            }
        }

        let p = Paragraph::new(Text::from(lines));
        f.render_widget(p, inner);
    } else {
        let p = Paragraph::new(app.qasm_text.as_str())
            .style(Style::default().fg(DARK_BLUE))
            .scroll((app.qasm_scroll, 0));
        f.render_widget(p, inner);
    }
}

// ── Controls Panel ─────────────────────────────────────────────────────────────

fn render_controls_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut help = match app.focus {
        Focus::Qasm => "QASM:  Tab Exit editor  Type to edit  q Quit".to_string(),
        _ => "Nav: ↑↓/jk Qubit  ←→/hl Step  +/- Qubits  a Add gate  Tab Focus  Bksp Del  e Edit  v Statevec  m Matrix  Ctrl+S Save  q Quit".to_string(),
    };

    if app.focus == Focus::Qasm {
        let (row, _) = app.qasm_cursor_row_col();
        if let Some((_, msg)) = app.qasm_errors.iter().find(|(r, _)| *r == row) {
            help = format!("ERROR: {}", msg);
        }
    }

    let p = Paragraph::new(Span::styled(help, Style::default().fg(YELLOW)));
    f.render_widget(p, inner);
}

// ── Menu Overlay ──────────────────────────────────────────────────────────────

fn render_menu_overlay(f: &mut Frame, app: &App) {
    let area = overlay_rect(f.area(), 75, 20);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            "Add Gate",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Category tabs
    let mut cat_line: Vec<Span> = Vec::new();
    for (i, cat) in GATE_MENU.iter().enumerate() {
        let name = format!(" {} ", cat.name);
        if i == app.menu_cat {
            cat_line.push(Span::styled(
                name,
                Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
            ));
        } else {
            cat_line.push(Span::styled(name, Style::default().fg(DIM)));
        }
        if i < GATE_MENU.len() - 1 {
            cat_line.push(Span::styled("│", Style::default().fg(DIM)));
        }
    }
    lines.push(Line::from(cat_line));
    lines.push(Line::styled("─".repeat(42), Style::default().fg(DIM)));

    // Items
    let cat = &GATE_MENU[app.menu_cat];
    for (i, item) in cat.items.iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();
        if i == app.menu_item {
            spans.push(Span::styled(
                " ▸ ",
                Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!("{:<18}", item.name),
                Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                item.symbol,
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw("   "));
            spans.push(Span::styled(
                format!("{:<18}", item.name),
                Style::default().fg(DARK_BLUE),
            ));
            spans.push(Span::styled(item.symbol, Style::default().fg(DIM)));
        }
        if item.needs_target {
            spans.push(Span::styled(" →target", Style::default().fg(DIM)));
        }
        if item.needs_params {
            if let Some(hint) = &item.param_hint {
                spans.push(Span::styled(
                    format!(" ({})", hint.example),
                    Style::default().fg(DIM),
                ));
            }
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::styled(
        "↑↓ Select  ←→ Cat  ⏎ Ok  Esc ✕",
        Style::default().fg(DIM),
    ));

    let p = Paragraph::new(Text::from(lines));
    f.render_widget(p, inner);
}

// ── Param Input Overlay ────────────────────────────────────────────────────────

fn render_param_input_overlay(f: &mut Frame, app: &App) {
    let area = overlay_rect(f.area(), 40, 7);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            "Enter Parameter",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::default(),
        Line::styled(
            format!("Value: {}_", app.param_input),
            Style::default().fg(DARK_BLUE),
        ),
        Line::default(),
        Line::styled("Examples: pi/2, 3*pi/4, 1.57", Style::default().fg(DIM)),
    ];

    let p = Paragraph::new(Text::from(lines));
    f.render_widget(p, inner);
}

// ── Edit Gate Overlay ──────────────────────────────────────────────────────────

fn render_edit_gate_overlay(f: &mut Frame, app: &App) {
    let area = overlay_rect(f.area(), 40, 12);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            "Edit Gate",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let opts = app.get_edit_options();
    let mut lines: Vec<Line> = vec![Line::default()];

    for (i, opt) in opts.iter().enumerate() {
        if i == app.edit_menu_idx {
            lines.push(Line::styled(
                format!("▸ {}", opt.label),
                Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
            ));
        } else {
            lines.push(Line::styled(
                format!("  {}", opt.label),
                Style::default().fg(DARK_BLUE),
            ));
        }
    }

    lines.push(Line::default());
    lines.push(Line::styled(
        "↑↓ Select  ⏎ Ok  Esc ✕",
        Style::default().fg(DIM),
    ));

    let p = Paragraph::new(Text::from(lines));
    f.render_widget(p, inner);
}

// ── Overlay rect helper ────────────────────────────────────────────────────────

fn overlay_rect(screen: Rect, min_w: u16, min_h: u16) -> Rect {
    let w = min_w.min(screen.width.saturating_sub(4));
    let h = min_h.min(screen.height.saturating_sub(4));
    Rect {
        x: 2,
        y: 2,
        width: w,
        height: h,
    }
}
