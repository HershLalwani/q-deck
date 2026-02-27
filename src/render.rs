use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus};
use crate::circuit::{CellInfo, Circuit};
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
    let qasm_w = ((size.width / 3) as usize).max(30).min((size.width - 20) as usize) as u16;
    let left_w = size.width.saturating_sub(qasm_w);

    let state_h = if avail_h < 20 { avail_h / 3 } else { 13 }.max(4).min(avail_h.saturating_sub(3));
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
    render_state_panel(f, app, left_chunks[1]);
    render_qasm_panel(f, app, top_chunks[1]);
    render_controls_panel(f, app, main_chunks[1]);

    // Overlays
    match app.focus {
        Focus::Menu => render_menu_overlay(f, app),
        Focus::InputParam => render_param_input_overlay(f, app),
        Focus::EditGate => render_edit_gate_overlay(f, app),
        _ => {}
    }
}

// ── Circuit Panel ─────────────────────────────────────────────────────────────

fn render_circuit_panel(f: &mut Frame, app: &App, area: Rect) {
    let active = matches!(
        app.focus,
        Focus::Circuit | Focus::SelectTarget | Focus::Menu | Focus::SelectControls
    );
    let border_color = if active { ORANGE } else { BLUE };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled("Quantum Circuit", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let circuit = app.circuit();
    let lines = build_circuit_lines(app, &circuit, inner.width as usize);

    let text: Vec<Line> = lines.into_iter().map(|l| Line::raw(l)).collect();
    let p = Paragraph::new(Text::from(text));
    f.render_widget(p, inner);
}

fn build_circuit_lines(app: &App, circuit: &Circuit, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // Header line
    let avail = width.saturating_sub(LABEL_W + 2);
    let max_steps = (avail / CELL_W).max(1);

    let start_step = if app.cursor_step >= max_steps as isize {
        (app.cursor_step - max_steps as isize + 1) as usize
    } else {
        0
    };

    // Step numbers header
    let mut step_hdr = " ".repeat(LABEL_W);
    for step in start_step..start_step + max_steps {
        step_hdr.push_str(&pad_center(&format!("{step}"), CELL_W));
    }
    lines.push(step_hdr);

    // Qubit rows
    for qubit in 0..circuit.num_qubits {
        let mut top_line = " ".repeat(LABEL_W);
        let label = format!("q[{qubit}]");
        let mut mid_line = format!("{:<5}", label) + "──";
        let mut bot_line = " ".repeat(LABEL_W);

        for step_idx in start_step..start_step + max_steps {
            let step = step_idx as isize;
            let info = circuit.get_cell_info(step, qubit);

            let is_cursor = step == app.cursor_step
                && qubit == app.cursor_qubit
                && matches!(
                    app.focus,
                    Focus::Circuit | Focus::SelectTarget | Focus::Menu | Focus::SelectControls
                );
            let is_target_sel = step == app.cursor_step
                && qubit == app.target_qubit
                && app.focus == Focus::SelectTarget;

            let (top, mid, bot) = render_cell(&info, is_cursor, is_target_sel, qubit);
            top_line.push_str(&top);
            mid_line.push_str(&mid);
            bot_line.push_str(&bot);
        }

        lines.push(top_line);
        lines.push(mid_line);
        lines.push(bot_line);
    }

    // Classical bit wire
    let num_cbits = circuit.num_cbits();
    if num_cbits > 0 {
        let mut sep = " ".repeat(LABEL_W);
        for step_idx in start_step..start_step + max_steps {
            let mq = circuit.get_measure_at_step(step_idx as isize);
            if mq >= 0 {
                let half = CELL_W / 2;
                sep.push_str(&" ".repeat(half));
                sep.push('║');
                sep.push_str(&" ".repeat(CELL_W - half - 1));
            } else {
                sep.push_str(&" ".repeat(CELL_W));
            }
        }
        lines.push(sep);

        let cbit_label = format!("c{num_cbits}");
        let mut cbit_line = format!("{:<5}", cbit_label) + "══";
        for step_idx in start_step..start_step + max_steps {
            let mq = circuit.get_measure_at_step(step_idx as isize);
            if mq >= 0 {
                let bit_label = format!("{mq}");
                let dash_l = (CELL_W - 1) / 2;
                let dash_r = CELL_W.saturating_sub(dash_l + 1 + bit_label.len());
                cbit_line.push_str(&"═".repeat(dash_l));
                cbit_line.push_str(&format!("╩{bit_label}"));
                cbit_line.push_str(&"═".repeat(dash_r));
            } else {
                cbit_line.push_str(&"═".repeat(CELL_W));
            }
        }
        lines.push(cbit_line);
    }

    // Status / position line
    match app.focus {
        Focus::SelectTarget => {
            lines.push(format!(
                "  {} Select target: q[{}]  ↑↓ Move  Enter Confirm  Esc Cancel",
                app.pending_gate, app.target_qubit
            ));
        }
        _ => {
            let mut status = format!("  Position: Step {}, Qubit {}", app.cursor_step, app.cursor_qubit);
            if !app.status_msg.is_empty() {
                status.push_str(&format!("  │  {}", app.status_msg));
            }
            lines.push(status);
        }
    }

    lines
}

fn render_cell(info: &CellInfo, is_cursor: bool, is_target_sel: bool, qubit: usize) -> (String, String, String) {
    let empty = " ".repeat(CELL_W);
    let half = CELL_W / 2;
    let vert_row = " ".repeat(half) + "│" + &" ".repeat(CELL_W - half - 1);
    let dbl_vert = " ".repeat(half) + "║" + &" ".repeat(CELL_W - half - 1);

    let dash_l = (CELL_W - 1) / 2;
    let dash_r = CELL_W - dash_l - 1;

    if is_cursor || is_target_sel {
        let inner_w = CELL_W - 2;
        let dleft = (inner_w - 1) / 2;
        let dright = inner_w - dleft - 1;
        let bdr_l = if is_cursor { "╔" } else { "╔" };
        let bdr_r = if is_cursor { "╗" } else { "╗" };
        let bdr_bl = if is_cursor { "╚" } else { "╚" };
        let bdr_br = if is_cursor { "╝" } else { "╝" };

        if info.is_barrier {
            let top = vert_row.clone();
            let mid = "║".to_string() + &"─".repeat(dleft) + "│" + &"─".repeat(dright) + "║";
            let bot = vert_row.clone();
            return (top, mid, bot);
        }

        let top = format!("{bdr_l}{}{bdr_r}", "═".repeat(inner_w));
        let bot = format!("{bdr_bl}{}{bdr_br}", "═".repeat(inner_w));

        let mid = if let Some(gate) = &info.gate {
            if info.is_control {
                let sym = control_symbol(&gate.type_name);
                format!("║{}{}{}║", "─".repeat(dleft), sym, "─".repeat(dright))
            } else if info.is_target {
                let sym = target_symbol(&gate.type_name);
                format!("║{}{}{}║", "─".repeat(dleft), sym, "─".repeat(dright))
            } else if gate.measure_source >= 0 {
                let sym = if gate.measure_source as usize == qubit { "M" } else { "⊕" };
                format!("║{}{}{}║", "─".repeat(dleft), sym, "─".repeat(dright))
            } else {
                let name = pad_center(&gate_display_name(&gate.type_name), GATE_NAME_W);
                format!("║─┤{}├─║", name)
            }
        } else if info.pass_through {
            format!("║{}┼{}║", "─".repeat(dleft), "─".repeat(dright))
        } else {
            format!("║{}║", "─".repeat(inner_w))
        };

        return (top, mid, bot);
    }

    // Normal cells
    if info.is_barrier {
        let top = vert_row.clone();
        let mid = "─".repeat(dash_l) + "│" + &"─".repeat(dash_r);
        let bot = vert_row.clone();
        return (top, mid, bot);
    }

    if let Some(gate) = &info.gate {
        if info.is_control {
            let top = if info.vert_above { vert_row.clone() } else { empty.clone() };
            let sym = control_symbol(&gate.type_name);
            let mid = "─".repeat(dash_l) + &sym + &"─".repeat(dash_r);
            let bot = if info.measure_below { dbl_vert.clone() } else if info.vert_below { vert_row.clone() } else { empty.clone() };
            return (top, mid, bot);
        }
        if info.is_target {
            let top = if info.vert_above { vert_row.clone() } else { empty.clone() };
            let sym = target_symbol(&gate.type_name);
            let mid = "─".repeat(dash_l) + &sym + &"─".repeat(dash_r);
            let bot = if info.measure_below { dbl_vert.clone() } else if info.vert_below { vert_row.clone() } else { empty.clone() };
            return (top, mid, bot);
        }
        if gate.measure_source >= 0 {
            let margin = (CELL_W - GATE_NAME_W - 2) / 2;
            let rmargin = CELL_W - margin - GATE_NAME_W - 2;
            if gate.measure_source as usize == qubit {
                let top = " ".repeat(margin) + "┌" + &"─".repeat(GATE_NAME_W) + "┐" + &" ".repeat(rmargin);
                let mid = "─".repeat(margin) + "┤" + &pad_center("M", GATE_NAME_W) + "├" + &"─".repeat(rmargin);
                let bot = if info.measure_below { dbl_vert } else { " ".repeat(margin) + "└" + &"─".repeat(GATE_NAME_W) + "┘" + &" ".repeat(rmargin) };
                return (top, mid, bot);
            } else if gate.target == qubit {
                let top = if info.vert_above { vert_row.clone() } else { empty.clone() };
                let mid = "─".repeat(dash_l) + "⊕" + &"─".repeat(dash_r);
                let bot = if info.measure_below { dbl_vert } else if info.vert_below { vert_row } else { empty };
                return (top, mid, bot);
            }
        }
        if gate.type_name == "MEASURE" {
            let margin = (CELL_W - GATE_NAME_W - 2) / 2;
            let rmargin = CELL_W - margin - GATE_NAME_W - 2;
            let top = " ".repeat(margin) + "┌" + &"─".repeat(GATE_NAME_W) + "┐" + &" ".repeat(rmargin);
            let mid = "─".repeat(margin) + "┤" + &pad_center("M", GATE_NAME_W) + "├" + &"─".repeat(rmargin);
            let bot = " ".repeat(margin) + "└" + &"─".repeat(GATE_NAME_W) + "┘" + &" ".repeat(rmargin);
            return (top, mid, bot);
        }
        // Normal single-qubit gate box
        let margin = (CELL_W - GATE_NAME_W - 2) / 2;
        let rmargin = CELL_W - margin - GATE_NAME_W - 2;
        let name = pad_center(&gate_display_name(&gate.type_name), GATE_NAME_W);
        let top = " ".repeat(margin) + "┌" + &"─".repeat(GATE_NAME_W) + "┐" + &" ".repeat(rmargin);
        let mid = "─".repeat(margin) + "┤" + &name + "├" + &"─".repeat(rmargin);
        let bot = if info.measure_below { dbl_vert } else { " ".repeat(margin) + "└" + &"─".repeat(GATE_NAME_W) + "┘" + &" ".repeat(rmargin) };
        return (top, mid, bot);
    }

    if info.pass_through {
        let top = vert_row.clone();
        let mid = "─".repeat(dash_l) + "┼" + &"─".repeat(dash_r);
        let bot = if info.measure_below { dbl_vert } else { vert_row };
        return (top, mid, bot);
    }

    if info.measure_below {
        let top = if info.vert_above { vert_row } else { dbl_vert.clone() };
        let mid = "─".repeat(dash_l) + "╫" + &"─".repeat(dash_r);
        let bot = dbl_vert;
        return (top, mid, bot);
    }

    // Empty wire
    let top = if info.vert_above { vert_row.clone() } else { empty.clone() };
    let mid = "─".repeat(CELL_W);
    let bot = if info.vert_below { vert_row.clone() } else { empty };
    (top, mid, bot)
}

fn control_symbol(gate_type: &str) -> String {
    if gate_type == "SWAP" { "×".to_string() } else { "●".to_string() }
}

fn target_symbol(gate_type: &str) -> String {
    match gate_type {
        "CZ" => "●".to_string(),
        "SWAP" => "×".to_string(),
        _ => "⊕".to_string(),
    }
}

fn gate_display_name(gate_type: &str) -> String {
    match gate_type {
        "MEASURE" => "M".to_string(),
        other => other.to_string(),
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
    let title = "Probabilities";

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let circuit = app.circuit();
    let state = simulate_circuit(&circuit, app.cursor_step);
    let mut qsphere = state.get_qsphere_states();
    qsphere.sort_by(|a, b| b.prob.partial_cmp(&a.prob).unwrap_or(std::cmp::Ordering::Equal));

    let num_qubits = circuit.num_qubits.max(app.dag.num_qubits).max(1);
    let bar_width = (inner.width as usize).saturating_sub(30).max(10);

    let mut text_lines: Vec<Line> = Vec::new();

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

// ── QASM Panel ────────────────────────────────────────────────────────────────

fn render_qasm_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Qasm;
    let border_color = if active { ORANGE } else { PURPLE };
    let title = if active { "QASM Editor [ACTIVE]" } else { "QASM Editor" };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)));

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
                    Span::styled(before, Style::default().fg(DARK_BLUE)),
                    Span::styled(cur_ch, Style::default().fg(Color::Black).bg(DARK_BLUE)),
                    Span::styled(after, Style::default().fg(DARK_BLUE)),
                ]));
            } else {
                lines.push(Line::styled(*line_str, Style::default().fg(DARK_BLUE)));
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

    let help = match app.focus {
        Focus::Qasm => "QASM:  Tab Exit editor  Type to edit  q Quit".to_string(),
        _ => "Nav: ↑↓/jk Qubit  ←→/hl Step  +/- Qubits  a Add gate  Tab Focus  Bksp Del  e Edit  Ctrl+S Save  q Quit".to_string(),
    };

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
        .title(Span::styled("Add Gate", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Category tabs
    let mut cat_line: Vec<Span> = Vec::new();
    for (i, cat) in GATE_MENU.iter().enumerate() {
        let name = format!(" {} ", cat.name);
        if i == app.menu_cat {
            cat_line.push(Span::styled(name, Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)));
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
            spans.push(Span::styled(" ▸ ", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(
                format!("{:<18}", item.name),
                Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(item.symbol, Style::default().fg(CYAN).add_modifier(Modifier::BOLD)));
        } else {
            spans.push(Span::raw("   "));
            spans.push(Span::styled(format!("{:<18}", item.name), Style::default().fg(DARK_BLUE)));
            spans.push(Span::styled(item.symbol, Style::default().fg(DIM)));
        }
        if item.needs_target {
            spans.push(Span::styled(" →target", Style::default().fg(DIM)));
        }
        if item.needs_params {
            if let Some(hint) = &item.param_hint {
                spans.push(Span::styled(format!(" ({})", hint.example), Style::default().fg(DIM)));
            }
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::styled("↑↓ Select  ←→ Cat  ⏎ Ok  Esc ✕", Style::default().fg(DIM)));

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
        .title(Span::styled("Enter Parameter", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::default(),
        Line::styled(format!("Value: {}_", app.param_input), Style::default().fg(DARK_BLUE)),
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
        .title(Span::styled("Edit Gate", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)));

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
            lines.push(Line::styled(format!("  {}", opt.label), Style::default().fg(DARK_BLUE)));
        }
    }

    lines.push(Line::default());
    lines.push(Line::styled("↑↓ Select  ⏎ Ok  Esc ✕", Style::default().fg(DIM)));

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
