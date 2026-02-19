package main

import (
	"fmt"
	"strings"
)

// ──────────────────────────── Rendering helpers ────────────────────────────

// padCenter centres a string within the given width.
func padCenter(s string, width int) string {
	if len(s) >= width {
		return s[:width]
	}
	total := width - len(s)
	left := total / 2
	right := total - left
	return strings.Repeat(" ", left) + s + strings.Repeat(" ", right)
}

// gateDisplayName returns a short display name for a gate type.
func gateDisplayName(gateType string) string {
	switch gateType {
	case "MEASURE":
		return "M"
	default:
		return gateType
	}
}

// controlSymbol returns the wire symbol for the control qubit of a two-qubit gate.
func controlSymbol(gateType string) string {
	if gateType == "SWAP" {
		return "×"
	}
	return "●"
}

// targetSymbol returns the wire symbol for the target qubit of a two-qubit gate.
func targetSymbol(gateType string) string {
	switch gateType {
	case "CZ":
		return "●"
	case "SWAP":
		return "×"
	default:
		return "⊕"
	}
}

// ──────────────────────────── Cell rendering ────────────────────────────

type cellHighlight int

const (
	hlNone cellHighlight = iota
	hlCursor
	hlTargetSelect
)

// renderCell returns 3 lines (top, mid, bot) for a single cell.
// Each line is exactly cellW (11) visual characters wide.
func renderCell(info cellInfo, hl cellHighlight, qubit int) (top, mid, bot string) {
	emptyRow := strings.Repeat(" ", cellW)
	halfW := cellW / 2
	vertRow := strings.Repeat(" ", halfW) + "│" + strings.Repeat(" ", cellW-halfW-1)
	dblVertRow := strings.Repeat(" ", halfW) + cbitConnectorStyle.Render("║") + strings.Repeat(" ", cellW-halfW-1)

	// ── Highlighted cell (cursor or target selection) ──
	if hl == hlCursor || hl == hlTargetSelect {
		bdr := cursorBoxStyle
		if hl == hlTargetSelect {
			bdr = targetSelectStyle
		}
		innerW := cellW - 2
		dashL := (innerW - 1) / 2
		dashR := innerW - dashL - 1

		if info.isBarrier {
			top = vertRow
			mid = bdr.Render("║") + strings.Repeat("─", dashL) + "│" + strings.Repeat("─", dashR) + bdr.Render("║")
			bot = vertRow
			return
		}

		top = bdr.Render("╔" + strings.Repeat("═", innerW) + "╗")
		bot = bdr.Render("╚" + strings.Repeat("═", innerW) + "╝")

		if info.gate != nil {
			if info.isControl {
				sym := controlSymbol(info.gate.Type)
				mid = bdr.Render("║") + strings.Repeat("─", dashL) + gateStyle.Render(sym) + strings.Repeat("─", dashR) + bdr.Render("║")
			} else if info.isTarget {
				sym := targetSymbol(info.gate.Type)
				mid = bdr.Render("║") + strings.Repeat("─", dashL) + gateStyle.Render(sym) + strings.Repeat("─", dashR) + bdr.Render("║")
			} else if info.gate.MeasureSource >= 0 {
				if info.gate.MeasureSource == qubit {
					mid = bdr.Render("║") + strings.Repeat("─", dashL) + gateStyle.Render("M") + strings.Repeat("─", dashR) + bdr.Render("║")
				} else {
					mid = bdr.Render("║") + strings.Repeat("─", dashL) + gateStyle.Render("⊕") + strings.Repeat("─", dashR) + bdr.Render("║")
				}
			} else {
				name := padCenter(gateDisplayName(info.gate.Type), gateNameW)
				mid = bdr.Render("║") + "─┤" + gateStyle.Render(name) + "├─" + bdr.Render("║")
			}
		} else if info.passThrough {
			mid = bdr.Render("║") + strings.Repeat("─", dashL) + "┼" + strings.Repeat("─", dashR) + bdr.Render("║")
		} else {
			mid = bdr.Render("║") + strings.Repeat("─", innerW) + bdr.Render("║")
		}

		return
	}

	// ── Normal (non-highlighted) cells ──
	dashL := (cellW - 1) / 2
	dashR := cellW - dashL - 1

	if info.isBarrier {
		top = vertRow
		mid = strings.Repeat("─", dashL) + "│" + strings.Repeat("─", dashR)
		bot = vertRow

	} else if info.gate != nil {
		if info.isControl {
			top = emptyRow
			if info.vertAbove {
				top = vertRow
			}
			sym := controlSymbol(info.gate.Type)
			mid = strings.Repeat("─", dashL) + gateStyle.Render(sym) + strings.Repeat("─", dashR)
			bot = emptyRow
			if info.vertBelow {
				bot = vertRow
			}
			if info.measureBelow {
				bot = dblVertRow
			}

		} else if info.isTarget {
			top = emptyRow
			if info.vertAbove {
				top = vertRow
			}
			sym := targetSymbol(info.gate.Type)
			mid = strings.Repeat("─", dashL) + gateStyle.Render(sym) + strings.Repeat("─", dashR)
			bot = emptyRow
			if info.vertBelow {
				bot = vertRow
			}
			if info.measureBelow {
				bot = dblVertRow
			}

		} else if info.gate.MeasureSource >= 0 {
			if info.gate.MeasureSource == qubit {
				margin := (cellW - gateBoxW) / 2
				rightMargin := cellW - margin - gateBoxW
				top = strings.Repeat(" ", margin) + gateStyle.Render("┌"+strings.Repeat("─", gateNameW)+"┐") + strings.Repeat(" ", rightMargin)
				mid = strings.Repeat("─", margin) + gateStyle.Render("┤"+padCenter("M", gateNameW)+"├") + strings.Repeat("─", rightMargin)
				bot = strings.Repeat(" ", margin) + gateStyle.Render("└"+strings.Repeat("─", gateNameW)+"┘") + strings.Repeat(" ", rightMargin)
				if info.measureBelow {
					bot = dblVertRow
				}
			} else if info.gate.Target == qubit {
				top = emptyRow
				if info.vertAbove {
					top = vertRow
				}
				mid = strings.Repeat("─", dashL) + gateStyle.Render("⊕") + strings.Repeat("─", dashR)
				bot = emptyRow
				if info.vertBelow {
					bot = vertRow
				}
				if info.measureBelow {
					bot = dblVertRow
				}
			}

		} else if info.gate.Type == "MEASURE" {
			margin := (cellW - gateBoxW) / 2
			rightMargin := cellW - margin - gateBoxW
			top = strings.Repeat(" ", margin) + gateStyle.Render("┌"+strings.Repeat("─", gateNameW)+"┐") + strings.Repeat(" ", rightMargin)
			mid = strings.Repeat("─", margin) + gateStyle.Render("┤"+padCenter("M", gateNameW)+"├") + strings.Repeat("─", rightMargin)
			bot = strings.Repeat(" ", margin) + gateStyle.Render("└"+strings.Repeat("─", gateNameW)+"┘") + strings.Repeat(" ", rightMargin)

		} else {
			margin := (cellW - gateBoxW) / 2
			rightMargin := cellW - margin - gateBoxW
			name := padCenter(gateDisplayName(info.gate.Type), gateNameW)

			top = strings.Repeat(" ", margin) + gateStyle.Render("┌"+strings.Repeat("─", gateNameW)+"┐") + strings.Repeat(" ", rightMargin)
			mid = strings.Repeat("─", margin) + gateStyle.Render("┤"+name+"├") + strings.Repeat("─", rightMargin)
			bot = strings.Repeat(" ", margin) + gateStyle.Render("└"+strings.Repeat("─", gateNameW)+"┘") + strings.Repeat(" ", rightMargin)
			if info.measureBelow {
				bot = dblVertRow
			}
		}

	} else if info.passThrough {
		top = vertRow
		mid = strings.Repeat("─", dashL) + "┼" + strings.Repeat("─", dashR)
		bot = vertRow
		if info.measureBelow {
			bot = dblVertRow
		}

	} else if info.measureBelow {
		// No gate here, but a measurement connection passes through vertically
		top = dblVertRow
		mid = strings.Repeat("─", dashL) + cbitConnectorStyle.Render("╫") + strings.Repeat("─", dashR)
		bot = dblVertRow
		if info.vertAbove {
			top = vertRow
		}

	} else {
		// Empty wire
		top = emptyRow
		if info.vertAbove {
			top = vertRow
		}
		mid = strings.Repeat("─", cellW)
		bot = emptyRow
		if info.vertBelow {
			bot = vertRow
		}
	}

	return
}

// ──────────────────────────── Panel rendering ────────────────────────────

// renderCircuitPanel renders the circuit grid panel.
func (m Model) renderCircuitPanel(width, height int) string {
	var sb strings.Builder

	sb.WriteString(titleStyle.Render("Quantum Circuit"))
	sb.WriteString("\n\n")

	// How many steps fit
	availWidth := width - labelVisualW - 4
	maxSteps := max(availWidth/cellW, 1)

	startStep := 0
	if m.cursorStep >= maxSteps {
		startStep = m.cursorStep - maxSteps + 1
	}

	displaySteps := maxSteps

	if startStep > 0 {
		fmt.Fprintf(&sb, "  ◀ showing steps %d–%d\n", startStep, startStep+displaySteps-1)
	}

	// Step number header
	header := strings.Repeat(" ", labelVisualW)
	for step := startStep; step < startStep+displaySteps; step++ {
		header += dimStyle.Render(padCenter(fmt.Sprintf("%d", step), cellW))
	}
	sb.WriteString(header + "\n")

	// Render each qubit as 3 lines
	for qubit := range m.circuit.NumQubits {
		topLine := strings.Repeat(" ", labelVisualW)
		label := fmt.Sprintf("q[%d]", qubit)
		midLine := qubitLabelStyle.Render(fmt.Sprintf("%-5s", label)) + "──"
		botLine := strings.Repeat(" ", labelVisualW)

		for step := startStep; step < startStep+displaySteps; step++ {
			info := m.circuit.getCellInfo(step, qubit)

			hl := hlNone
			if step == m.cursorStep && qubit == m.cursorQubit && (m.focus == focusCircuit || m.focus == focusSelectTarget || m.focus == focusMenu) {
				hl = hlCursor
			} else if step == m.cursorStep && qubit == m.targetQubit && m.focus == focusSelectTarget {
				hl = hlTargetSelect
			}

			top, mid, bot := renderCell(info, hl, qubit)
			topLine += top
			midLine += mid
			botLine += bot
		}

		sb.WriteString(topLine + "\n")
		sb.WriteString(midLine + "\n")
		sb.WriteString(botLine + "\n")
	}

	// ── Classical bit wire (single line) ──
	numCbits := m.circuit.NumCbits()
	if numCbits > 0 {
		// Separator line between quantum and classical wires
		sepLine := strings.Repeat(" ", labelVisualW)
		for step := startStep; step < startStep+displaySteps; step++ {
			measuredQubit := m.circuit.GetMeasureAtStep(step)
			halfW := cellW / 2
			if measuredQubit >= 0 {
				sepLine += strings.Repeat(" ", halfW) + cbitConnectorStyle.Render("║") + strings.Repeat(" ", cellW-halfW-1)
			} else {
				sepLine += strings.Repeat(" ", cellW)
			}
		}
		sb.WriteString(sepLine + "\n")

		// Single classical wire showing count and measurement landing points
		label := fmt.Sprintf("c%d", numCbits)
		cbitLine := cbitLabelStyle.Render(fmt.Sprintf("%-5s", label)) + cbitWireStyle.Render("══")

		for step := startStep; step < startStep+displaySteps; step++ {
			measuredQubit := m.circuit.GetMeasureAtStep(step)
			if measuredQubit >= 0 {
				// Show ╩ with the bit index next to it
				bitLabel := fmt.Sprintf("%d", measuredQubit)
				dashL := (cellW - 1) / 2
				dashR := max(cellW-dashL-1-len(bitLabel), 0)
				cbitLine += cbitWireStyle.Render(strings.Repeat("═", dashL)) +
					cbitConnectorStyle.Render("╩"+bitLabel) +
					cbitWireStyle.Render(strings.Repeat("═", dashR))
			} else {
				cbitLine += cbitWireStyle.Render(strings.Repeat("═", cellW))
			}
		}
		sb.WriteString(cbitLine + "\n")
	}

	// Status line
	if m.focus == focusSelectTarget {
		sb.WriteString("\n")
		fmt.Fprintf(&sb, "  %s", activeGateStyle.Render(m.pendingGate))
		sb.WriteString("  Select target qubit: ")
		fmt.Fprintf(&sb, "%s", targetSelectStyle.Render(fmt.Sprintf("q[%d]", m.targetQubit)))
		sb.WriteString(dimStyle.Render("   ↑↓ Move  Enter Confirm  Esc Cancel"))
	} else {
		fmt.Fprintf(&sb, "\n  Position: Step %d, Qubit %d", m.cursorStep, m.cursorQubit)
		if m.statusMsg != "" {
			fmt.Fprintf(&sb, "  │  %s", activeGateStyle.Render(m.statusMsg))
		}
	}

	return circuitStyle.Width(width).Height(height).Render(sb.String())
}

// renderQASMPanel renders the QASM editor panel.
func (m Model) renderQASMPanel(width, height int) string {
	var sb strings.Builder

	title := "QASM Editor"
	if m.focus == focusQASM {
		title += " [ACTIVE]"
	}
	sb.WriteString(titleStyle.Render(title))
	sb.WriteString("\n\n")
	sb.WriteString(m.qasmEditor.View())

	return qasmStyle.Width(width).Height(height).Render(sb.String())
}

// renderControlsPanel renders the bottom help/controls bar.
func (m Model) renderControlsPanel(width, height int) string {
	var sb strings.Builder

	sb.WriteString(activeGateStyle.Render("Navigate: "))
	sb.WriteString("↑↓/jk Move qubit  ←→/hl Move step  +/- Qubits")
	sb.WriteString("    ")
	sb.WriteString(activeGateStyle.Render("a"))
	sb.WriteString(" Add gate\n")

	sb.WriteString(activeGateStyle.Render("Actions:  "))
	sb.WriteString("Tab Switch focus  Bksp Delete  ^R Reset  ^S Save  q/^C Quit")

	return controlsStyle.Width(width).Height(height).Render(sb.String())
}

// ──────────────────────────── Overlay helpers ────────────────────────────

// overlayAt composites the overlay string on top of the background at position (x, y).
// It handles ANSI escape sequences by tracking visible column positions.
func overlayAt(bg, overlay string, x, y int) string {
	bgLines := strings.Split(bg, "\n")
	ovLines := strings.Split(overlay, "\n")

	for i, ovLine := range ovLines {
		bgIdx := y + i
		if bgIdx < 0 || bgIdx >= len(bgLines) {
			continue
		}
		bgLines[bgIdx] = spliceLineAt(bgLines[bgIdx], ovLine, x)
	}
	return strings.Join(bgLines, "\n")
}

// spliceLineAt replaces visible columns starting at position x in bgLine with overlay content.
// It properly handles ANSI escape sequences in the background line.
func spliceLineAt(bgLine, overlay string, x int) string {
	runes := []rune(bgLine)
	ovWidth := visibleLen(overlay)

	var prefix strings.Builder
	var suffix strings.Builder

	col := 0
	i := 0
	inEsc := false

	// Collect prefix: everything up to visible column x
	for i < len(runes) && col < x {
		if runes[i] == '\x1b' {
			inEsc = true
			for i < len(runes) {
				prefix.WriteRune(runes[i])
				if inEsc && runes[i] != '\x1b' && runes[i] != '[' && ((runes[i] >= 'A' && runes[i] <= 'Z') || (runes[i] >= 'a' && runes[i] <= 'z')) {
					inEsc = false
					i++
					break
				}
				i++
			}
		} else {
			prefix.WriteRune(runes[i])
			col++
			i++
		}
	}

	// Pad prefix if bg line is shorter than x
	for col < x {
		prefix.WriteRune(' ')
		col++
	}

	// Skip over ovWidth visible columns in the background
	skipped := 0
	for i < len(runes) && skipped < ovWidth {
		if runes[i] == '\x1b' {
			for i < len(runes) {
				i++
				if i > 0 && runes[i-1] != '\x1b' && runes[i-1] != '[' && ((runes[i-1] >= 'A' && runes[i-1] <= 'Z') || (runes[i-1] >= 'a' && runes[i-1] <= 'z')) {
					break
				}
			}
		} else {
			skipped++
			i++
		}
	}

	// Collect suffix: rest of the background line
	for i < len(runes) {
		suffix.WriteRune(runes[i])
		i++
	}

	return prefix.String() + overlay + suffix.String()
}

// visibleLen returns the number of visible (non-ANSI-escape) characters in a string.
func visibleLen(s string) int {
	n := 0
	inEsc := false
	for _, r := range s {
		if r == '\x1b' {
			inEsc = true
			continue
		}
		if inEsc {
			if (r >= 'A' && r <= 'Z') || (r >= 'a' && r <= 'z') {
				inEsc = false
			}
			continue
		}
		n++
	}
	return n
}
