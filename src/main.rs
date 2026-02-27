pub mod circuit;
pub mod dag;
pub mod menu;
pub mod params;
pub mod quantum;
pub mod app;
pub mod render;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, Focus};

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), io::Error> {
    loop {
        terminal.draw(|f| render::render(f, app))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let evt = event::read()?;
        if let Event::Key(key) = evt {
            // Clear status message on any key
            app.status_msg.clear();

            let code = key.code;
            let mods = key.modifiers;

            // Global: Ctrl+C always quits
            if code == KeyCode::Char('c') && mods.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }

            match app.focus {
                Focus::Circuit => {
                    if handle_circuit_keys(app, code, mods) {
                        return Ok(());
                    }
                }
                Focus::Qasm => {
                    match code {
                        KeyCode::Tab => {
                            app.focus = Focus::Circuit;
                            app.parse_qasm_input();
                        }
                        KeyCode::Left => app.qasm_move_left(),
                        KeyCode::Right => app.qasm_move_right(),
                        KeyCode::Up => app.qasm_move_up(),
                        KeyCode::Down => app.qasm_move_down(),
                        KeyCode::Home => app.qasm_move_home(),
                        KeyCode::End => app.qasm_move_end(),
                        KeyCode::Backspace => {
                            app.qasm_backspace();
                            app.parse_qasm_input();
                        }
                        KeyCode::Delete => {
                            app.qasm_delete_forward();
                            app.parse_qasm_input();
                        }
                        KeyCode::Enter => {
                            app.qasm_insert_char('\n');
                            app.parse_qasm_input();
                        }
                        KeyCode::Char(c) => {
                            app.qasm_insert_char(c);
                            app.parse_qasm_input();
                        }
                        _ => {}
                    }
                }
                Focus::Menu => handle_menu_keys(app, code),
                Focus::SelectTarget => handle_select_target_keys(app, code),
                Focus::SelectControls => handle_select_controls_keys(app, code),
                Focus::InputParam => handle_input_param_keys(app, code),
                Focus::EditGate => handle_edit_gate_keys(app, code),
                Focus::EditParam => handle_edit_param_keys(app, code),
                Focus::EditTarget => handle_edit_target_keys(app, code),
                Focus::EditControl => handle_edit_control_keys(app, code),
            }
        }
    }
}

// ── Focus::Circuit ─────────────────────────────────────────────────────────────

fn handle_circuit_keys(app: &mut App, code: KeyCode, mods: KeyModifiers) -> bool {
    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Tab => {
            app.focus = Focus::Qasm;
        }
        KeyCode::Char('s') if mods.contains(KeyModifiers::CONTROL) => {
            match app.save_circuit() {
                Ok(()) => app.status_msg = "Saved circuit.qasm".to_string(),
                Err(e) => app.status_msg = format!("Save error: {e}"),
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.cursor_qubit > 0 {
                app.cursor_qubit -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.cursor_qubit + 1 < app.dag.num_qubits {
                app.cursor_qubit += 1;
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.cursor_step > 0 {
                app.cursor_step -= 1;
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.cursor_step += 1;
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            app.dag.num_qubits += 1;
            app.sync_from_dag();
        }
        KeyCode::Char('-') => {
            if app.dag.num_qubits > 1 {
                let q = app.dag.num_qubits - 1;
                app.dag.remove_nodes_on_qubit(q);
                app.dag.num_qubits -= 1;
                if app.cursor_qubit >= app.dag.num_qubits {
                    app.cursor_qubit = app.dag.num_qubits.saturating_sub(1);
                }
                app.sync_from_dag();
            }
        }
        KeyCode::Char('a') => {
            app.focus = Focus::Menu;
            app.menu_cat = 0;
            app.menu_item = 0;
        }
        KeyCode::Backspace | KeyCode::Delete => {
            app.dag.remove_node_at(app.cursor_step, app.cursor_qubit);
            app.sync_from_dag();
        }
        KeyCode::Char('e') => {
            let node = app.dag.get_node_at(app.cursor_step, app.cursor_qubit).cloned();
            if let Some(node) = node {
                let gate = crate::circuit::Gate {
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
                app.edit_gate = Some(gate);
                app.edit_menu_idx = 0;
                app.edit_orig_step = app.cursor_step;
                app.focus = Focus::EditGate;
            }
        }
        KeyCode::Char('v') => {
            app.show_statevector = !app.show_statevector;
        }
        _ => {}
    }
    false
}

// ── Focus::Menu ────────────────────────────────────────────────────────────────

fn handle_menu_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.focus = Focus::Circuit,
        KeyCode::Up | KeyCode::Char('k') => {
            if app.menu_item > 0 {
                app.menu_item -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = menu::GATE_MENU[app.menu_cat].items.len().saturating_sub(1);
            if app.menu_item < max {
                app.menu_item += 1;
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.menu_cat > 0 {
                app.menu_cat -= 1;
                app.menu_item = 0;
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.menu_cat + 1 < menu::GATE_MENU.len() {
                app.menu_cat += 1;
                app.menu_item = 0;
            }
        }
        KeyCode::Enter => {
            let item = &crate::menu::GATE_MENU[app.menu_cat].items[app.menu_item];
            let gate_type = item.gate_type.to_string();
            app.pending_gate = gate_type.clone();

            if crate::menu::is_parameterized_gate(&gate_type) {
                app.param_input.clear();
                app.focus = Focus::InputParam;
                return;
            }

            if gate_type == "CCX" {
                if app.dag.num_qubits < 3 {
                    app.focus = Focus::Circuit;
                    return;
                }
                app.control_qubits.clear();
                app.focus = Focus::SelectControls;
                app.target_qubit = if app.cursor_qubit + 1 < app.dag.num_qubits {
                    app.cursor_qubit + 1
                } else {
                    app.cursor_qubit.saturating_sub(1)
                };
                return;
            }

            if item.needs_target {
                if app.dag.num_qubits < 2 {
                    app.focus = Focus::Circuit;
                    return;
                }
                app.focus = Focus::SelectTarget;
                app.target_qubit = if app.cursor_qubit + 1 < app.dag.num_qubits {
                    app.cursor_qubit + 1
                } else {
                    app.cursor_qubit.saturating_sub(1)
                };
            } else {
                if app.place_gate(&gate_type.clone(), -1) {
                    app.focus = Focus::Circuit;
                }
            }
        }
        _ => {}
    }
}

// ── Focus::SelectTarget ─────────────────────────────────────────────────────────

fn handle_select_target_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.focus = Focus::Circuit;
            app.param_input.clear();
            app.control_qubits.clear();
            app.pending_gate.clear();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let excluded: Vec<usize> = std::iter::once(app.cursor_qubit)
                .chain(app.control_qubits.iter().cloned())
                .collect();
            if let Some(next) = app.next_available_target(app.target_qubit, -1, &excluded) {
                app.target_qubit = next;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let excluded: Vec<usize> = std::iter::once(app.cursor_qubit)
                .chain(app.control_qubits.iter().cloned())
                .collect();
            if let Some(next) = app.next_available_target(app.target_qubit, 1, &excluded) {
                app.target_qubit = next;
            }
        }
        KeyCode::Enter => {
            let gate = app.pending_gate.clone();
            if app.place_gate(&gate, app.target_qubit as isize) {
                app.focus = Focus::Circuit;
            }
        }
        _ => {}
    }
}

// ── Focus::SelectControls ───────────────────────────────────────────────────────

fn handle_select_controls_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.focus = Focus::Circuit;
            app.param_input.clear();
            app.control_qubits.clear();
            app.pending_gate.clear();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let excluded = vec![app.cursor_qubit];
            if let Some(next) = app.next_available_target(app.target_qubit, -1, &excluded) {
                app.target_qubit = next;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let excluded = vec![app.cursor_qubit];
            if let Some(next) = app.next_available_target(app.target_qubit, 1, &excluded) {
                app.target_qubit = next;
            }
        }
        KeyCode::Enter => {
            app.control_qubits.push(app.target_qubit);
            app.focus = Focus::SelectTarget;
            // Find a free qubit for target
            let excluded: Vec<usize> = std::iter::once(app.cursor_qubit)
                .chain(app.control_qubits.iter().cloned())
                .collect();
            for q in 0..app.dag.num_qubits {
                if !excluded.contains(&q) {
                    app.target_qubit = q;
                    break;
                }
            }
        }
        _ => {}
    }
}

// ── Focus::InputParam ──────────────────────────────────────────────────────────

fn handle_input_param_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.focus = Focus::Circuit;
            app.param_input.clear();
            app.pending_gate.clear();
        }
        KeyCode::Backspace => {
            app.param_input.pop();
        }
        KeyCode::Enter => {
            // Validate params
            if !app.param_input.is_empty() {
                if crate::params::parse_params(&app.param_input).is_none() {
                    app.status_msg = "Invalid parameter — use numbers or pi expressions (e.g. pi/2, 3*pi/4)".to_string();
                    return;
                }
            }
            let item = &crate::menu::GATE_MENU[app.menu_cat].items[app.menu_item];
            if item.needs_target {
                if app.dag.num_qubits < 2 {
                    app.focus = Focus::Circuit;
                    return;
                }
                app.focus = Focus::SelectTarget;
                app.target_qubit = if app.cursor_qubit + 1 < app.dag.num_qubits {
                    app.cursor_qubit + 1
                } else {
                    app.cursor_qubit.saturating_sub(1)
                };
            } else {
                let gate = app.pending_gate.clone();
                if app.place_gate(&gate, -1) {
                    app.focus = Focus::Circuit;
                }
            }
        }
        KeyCode::Char(c) => app.handle_char_input(c),
        _ => {}
    }
}

// ── Focus::EditGate ────────────────────────────────────────────────────────────

fn handle_edit_gate_keys(app: &mut App, code: KeyCode) {
    if app.edit_gate.is_none() {
        app.focus = Focus::Circuit;
        return;
    }
    let opts = app.get_edit_options();
    match code {
        KeyCode::Esc => {
            app.focus = Focus::Circuit;
            app.edit_gate = None;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.edit_menu_idx > 0 {
                app.edit_menu_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.edit_menu_idx + 1 < opts.len() {
                app.edit_menu_idx += 1;
            }
        }
        KeyCode::Enter => {
            if app.edit_menu_idx < opts.len() {
                let action = opts[app.edit_menu_idx].action;
                let ctrl_idx = opts[app.edit_menu_idx].ctrl_idx;
                match action {
                    "edit_param" => {
                        app.param_input.clear();
                        app.focus = Focus::EditParam;
                    }
                    "edit_target" => {
                        if let Some(g) = &app.edit_gate {
                            app.target_qubit = g.target;
                        }
                        app.focus = Focus::EditTarget;
                    }
                    "edit_control" => {
                        app.edit_control_idx = ctrl_idx;
                        if let Some(g) = &app.edit_gate {
                            app.target_qubit = if ctrl_idx == -1 {
                                g.control.max(0) as usize
                            } else if (ctrl_idx as usize) < g.controls.len() {
                                g.controls[ctrl_idx as usize]
                            } else {
                                0
                            };
                        }
                        app.focus = Focus::EditControl;
                    }
                    "delete" => {
                        let step = app.edit_orig_step;
                        if let Some(g) = &app.edit_gate {
                            let target = g.target;
                            app.dag.remove_node_at(step, target);
                        }
                        app.edit_gate = None;
                        app.focus = Focus::Circuit;
                        app.sync_from_dag();
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

// ── Focus::EditParam ───────────────────────────────────────────────────────────

fn handle_edit_param_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.param_input.clear();
            app.focus = Focus::EditGate;
        }
        KeyCode::Backspace => {
            app.param_input.pop();
        }
        KeyCode::Enter => {
            if !app.param_input.is_empty() {
                if let Some(params) = crate::params::parse_params(&app.param_input) {
                    if let Some(g) = &mut app.edit_gate {
                        g.params = params;
                    }
                } else {
                    app.status_msg = "Invalid parameter — use numbers or pi expressions".to_string();
                    return;
                }
            }
            app.param_input.clear();
            // Commit edit back to DAG
            commit_edit_to_dag(app);
            app.focus = Focus::EditGate;
        }
        KeyCode::Char(c) => app.handle_char_input(c),
        _ => {}
    }
}

// ── Focus::EditTarget ──────────────────────────────────────────────────────────

fn handle_edit_target_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.focus = Focus::EditGate,
        KeyCode::Up | KeyCode::Char('k') => {
            let excluded: Vec<usize> = app.edit_gate.as_ref().map(|g| {
                let mut v = vec![];
                if g.control >= 0 { v.push(g.control as usize); }
                v.extend_from_slice(&g.controls);
                v
            }).unwrap_or_default();
            if let Some(next) = app.next_available_target(app.target_qubit, -1, &excluded) {
                app.target_qubit = next;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let excluded: Vec<usize> = app.edit_gate.as_ref().map(|g| {
                let mut v = vec![];
                if g.control >= 0 { v.push(g.control as usize); }
                v.extend_from_slice(&g.controls);
                v
            }).unwrap_or_default();
            if let Some(next) = app.next_available_target(app.target_qubit, 1, &excluded) {
                app.target_qubit = next;
            }
        }
        KeyCode::Enter => {
            if let Some(g) = &mut app.edit_gate {
                g.target = app.target_qubit;
            }
            commit_edit_to_dag(app);
            app.focus = Focus::EditGate;
        }
        _ => {}
    }
}

// ── Focus::EditControl ─────────────────────────────────────────────────────────

fn handle_edit_control_keys(app: &mut App, code: KeyCode) {
    let unavailable: Vec<usize> = app.edit_gate.as_ref().map(|g| {
        let mut v = vec![g.target];
        let ci = app.edit_control_idx;
        for (i, &cq) in g.controls.iter().enumerate() {
            if i as isize != ci {
                v.push(cq);
            }
        }
        v
    }).unwrap_or_default();

    match code {
        KeyCode::Esc => app.focus = Focus::EditGate,
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(next) = app.next_available_target(app.target_qubit, -1, &unavailable) {
                app.target_qubit = next;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(next) = app.next_available_target(app.target_qubit, 1, &unavailable) {
                app.target_qubit = next;
            }
        }
        KeyCode::Enter => {
            let ci = app.edit_control_idx;
            if let Some(g) = &mut app.edit_gate {
                if ci == -1 {
                    g.control = app.target_qubit as isize;
                } else if (ci as usize) < g.controls.len() {
                    g.controls[ci as usize] = app.target_qubit;
                }
            }
            commit_edit_to_dag(app);
            app.focus = Focus::EditGate;
        }
        _ => {}
    }
}

// ── Edit commit helper ─────────────────────────────────────────────────────────

fn commit_edit_to_dag(app: &mut App) {
    if let Some(gate) = app.edit_gate.clone() {
        // Remove the old node
        app.dag.remove_node_at(app.edit_orig_step, gate.target);

        // Re-add with updated values
        if !gate.controls.is_empty() {
            app.dag.add_multi_control_gate(&gate.type_name, gate.target, app.edit_orig_step, gate.controls.clone());
        } else if gate.control >= 0 {
            if gate.params.is_empty() {
                app.dag.add_gate(&gate.type_name, gate.target, app.edit_orig_step, Some(gate.control as usize));
            } else {
                app.dag.add_parameterized_gate(&gate.type_name, gate.target, app.edit_orig_step, gate.params.clone(), Some(gate.control as usize));
            }
        } else if !gate.params.is_empty() {
            app.dag.add_parameterized_gate(&gate.type_name, gate.target, app.edit_orig_step, gate.params.clone(), None);
        } else if gate.is_reset {
            app.dag.add_reset(gate.target, app.edit_orig_step);
        } else if gate.is_dagger {
            app.dag.add_dagger_gate(&gate.type_name, gate.target, app.edit_orig_step);
        } else if gate.measure_source >= 0 {
            app.dag.add_measure_control_gate(gate.measure_source as usize, gate.target, app.edit_orig_step);
        } else {
            app.dag.add_gate(&gate.type_name, gate.target, app.edit_orig_step, None);
        }

        // Update edit_gate to reflect the new state
        app.edit_gate = Some(gate);
        app.sync_from_dag();
    }
}


