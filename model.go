package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/charmbracelet/bubbles/textarea"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// focus represents which panel/mode has keyboard input.
type focus int

const (
	focusCircuit focus = iota
	focusQASM
	focusMenu
	focusSelectTarget
)

// Model represents the TUI application state.
type Model struct {
	circuit     Circuit
	cursorQubit int
	cursorStep  int
	width       int
	height      int
	qasmEditor  textarea.Model
	focus       focus
	lastQASM    string
	statusMsg   string // transient status message (e.g. save confirmation)

	// Menu state
	menuCat  int
	menuItem int

	// Target-selection state (for multi-qubit gates)
	pendingGate string
	targetQubit int
}

func initialModel() Model {
	ta := textarea.New()
	ta.Placeholder = "Edit QASM here..."
	ta.SetWidth(40)
	ta.SetHeight(20)
	ta.ShowLineNumbers = true
	ta.KeyMap.InsertNewline.SetEnabled(true)

	m := Model{
		circuit: Circuit{
			NumQubits: 4,
		},
		qasmEditor: ta,
		focus:      focusCircuit,
	}

	m.syncQASM()
	return m
}

func (m *Model) syncQASM() {
	qasm := m.circuit.ToQASM()
	m.qasmEditor.SetValue(qasm)
	m.lastQASM = qasm
}

func (m *Model) parseQASMInput() {
	qasm := m.qasmEditor.Value()
	if qasm != m.lastQASM {
		m.circuit.ParseQASM(qasm)
		m.lastQASM = qasm
	}
}

// placeGate places a gate on the circuit at the cursor position.
// targetQ is the target qubit for multi-qubit gates (-1 for single-qubit).
func (m *Model) placeGate(gateType string, targetQ int) {
	m.circuit.RemoveGateAt(m.cursorStep, m.cursorQubit)
	if targetQ >= 0 {
		m.circuit.RemoveGateAt(m.cursorStep, targetQ)
	}

	switch gateType {
	case "CX", "CZ", "SWAP":
		m.circuit.AddGate(gateType, targetQ, m.cursorStep, m.cursorQubit)
	case "MCX":
		m.circuit.AddMeasureControlGate(m.cursorQubit, targetQ, m.cursorStep)
	case "MEASURE":
		m.circuit.AddGate("MEASURE", m.cursorQubit, m.cursorStep)
	default:
		// Single-qubit gate
		m.circuit.AddGate(gateType, m.cursorQubit, m.cursorStep)
	}

	m.cursorStep++
	m.circuit.MaxSteps = max(m.circuit.MaxSteps, m.cursorStep)
	m.syncQASM()
}

// ──────────────────────────── Init / Update ────────────────────────────

func (m Model) Init() tea.Cmd {
	return nil
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		qasmW := max(msg.Width/3-6, 20)
		m.qasmEditor.SetWidth(qasmW)
		ctrlH := 6
		circH := msg.Height - ctrlH - 4
		editorH := max(circH-8, 4)
		m.qasmEditor.SetHeight(editorH)

	case tea.KeyMsg:
		key := msg.String()
		m.statusMsg = "" // clear transient status on any keypress

		// ── Global shortcuts ──
		if key == "ctrl+c" {
			return m, tea.Quit
		}

		// ── Dispatch by focus ──
		switch m.focus {

		// ────────── Circuit mode ──────────
		case focusCircuit:
			switch key {
			case "q":
				return m, tea.Quit
			case "tab":
				m.focus = focusQASM
				m.qasmEditor.Focus()
			case "ctrl+r":
				m.circuit.Gates = nil
				m.circuit.MaxSteps = 0
				m.syncQASM()
			case "ctrl+s":
				qasm := m.circuit.ToQASM()
				if err := os.WriteFile("circuit.qasm", []byte(qasm), 0644); err != nil {
					m.statusMsg = fmt.Sprintf("Save error: %v", err)
				} else {
					m.statusMsg = "Saved circuit.qasm"
				}
			case "up", "k":
				if m.cursorQubit > 0 {
					m.cursorQubit--
				}
			case "down", "j":
				if m.cursorQubit < m.circuit.NumQubits-1 {
					m.cursorQubit++
				}
			case "left", "h":
				if m.cursorStep > 0 {
					m.cursorStep--
				}
			case "right", "l":
				m.cursorStep++
				m.circuit.MaxSteps = max(m.circuit.MaxSteps, m.cursorStep)
			case "+", "=":
				m.circuit.NumQubits++
				m.syncQASM()
			case "-":
				if m.circuit.NumQubits > 1 {
					m.circuit.NumQubits--
					m.cursorQubit = min(m.cursorQubit, m.circuit.NumQubits-1)
					// Remove any gates that reference the removed qubit
					m.circuit.RemoveGatesOnQubit(m.circuit.NumQubits)
					m.syncQASM()
				}
			case "a":
				m.focus = focusMenu
				m.menuCat = 0
				m.menuItem = 0
			case "backspace", "delete":
				m.circuit.RemoveGateAt(m.cursorStep, m.cursorQubit)
				m.syncQASM()
			}

		// ────────── Menu mode ──────────
		case focusMenu:
			switch key {
			case "esc":
				m.focus = focusCircuit
			case "up", "k":
				if m.menuItem > 0 {
					m.menuItem--
				}
			case "down", "j":
				cat := gateMenu[m.menuCat]
				if m.menuItem < len(cat.items)-1 {
					m.menuItem++
				}
			case "left", "h":
				if m.menuCat > 0 {
					m.menuCat--
					m.menuItem = 0
				}
			case "right", "l":
				if m.menuCat < len(gateMenu)-1 {
					m.menuCat++
					m.menuItem = 0
				}
			case "enter":
				item := gateMenu[m.menuCat].items[m.menuItem]
				if item.needsTarget {
					if m.circuit.NumQubits < 2 {
						// Not enough qubits
						break
					}
					m.pendingGate = item.gateType
					m.focus = focusSelectTarget
					// Default target: next qubit (or previous if at the end)
					m.targetQubit = m.cursorQubit + 1
					if m.targetQubit >= m.circuit.NumQubits {
						m.targetQubit = m.cursorQubit - 1
					}
				} else {
					m.placeGate(item.gateType, -1)
					m.focus = focusCircuit
				}
			}

		// ────────── Target-selection mode ──────────
		case focusSelectTarget:
			switch key {
			case "esc":
				m.focus = focusCircuit
			case "up", "k":
				for next := m.targetQubit - 1; next >= 0; next-- {
					if next != m.cursorQubit {
						m.targetQubit = next
						break
					}
				}
			case "down", "j":
				for next := m.targetQubit + 1; next < m.circuit.NumQubits; next++ {
					if next != m.cursorQubit {
						m.targetQubit = next
						break
					}
				}
			case "enter":
				m.placeGate(m.pendingGate, m.targetQubit)
				m.focus = focusCircuit
			}

		// ────────── QASM editor mode ──────────
		case focusQASM:
			switch key {
			case "tab":
				m.focus = focusCircuit
				m.qasmEditor.Blur()
			default:
				var cmd tea.Cmd
				m.qasmEditor, cmd = m.qasmEditor.Update(msg)
				cmds = append(cmds, cmd)
				m.parseQASMInput()
			}
		}
	}

	return m, tea.Batch(cmds...)
}

// ──────────────────────────── View ────────────────────────────

func (m Model) View() string {
	if m.width == 0 {
		return "Loading..."
	}

	// Normal layout
	qasmWidth := m.width / 3
	circuitWidth := m.width - qasmWidth - 4
	controlsHeight := 6
	circuitHeight := max(m.height-controlsHeight-2, 6)

	circuitPanel := m.renderCircuitPanel(circuitWidth, circuitHeight)
	qasmPanel := m.renderQASMPanel(qasmWidth, circuitHeight)
	controlsPanel := m.renderControlsPanel(m.width-4, controlsHeight-2)

	topRow := lipgloss.JoinHorizontal(lipgloss.Top, circuitPanel, qasmPanel)
	frame := lipgloss.JoinVertical(lipgloss.Left, topRow, controlsPanel)

	// When the gate menu is open, overlay it at the cursor cell position
	if m.focus == focusMenu {
		menuBox := m.renderMenu()

		// Calculate the cursor cell's screen position.
		// Circuit panel has: 1 border + 1 padding on the left.
		// Inside: title line, blank line, optional scroll indicator, header line,
		// then 3 lines per qubit (top/mid/bot).
		panelLeftPad := 2 // border(1) + padding(1)

		// How many steps fit (same logic as renderCircuitPanel)
		availWidth := circuitWidth - labelVisualW - 4
		maxSteps := max(availWidth/cellW, 1)
		startStep := 0
		if m.cursorStep >= maxSteps {
			startStep = m.cursorStep - maxSteps + 1
		}

		// X position: panel left pad + label area + cell offset within visible steps
		visibleStepIdx := m.cursorStep - startStep
		cellX := panelLeftPad + labelVisualW + visibleStepIdx*cellW

		// Y position: border(1) + padding(1) + title(1) + blank(1) + optional scroll(0 or 1) + header(1) + qubit*3 lines
		scrollLine := 0
		if startStep > 0 {
			scrollLine = 1
		}
		cellY := 2 + 2 + scrollLine + 1 + m.cursorQubit*3

		// Clamp so the menu doesn't overflow the screen edges
		menuLines := strings.Split(menuBox, "\n")
		menuH := len(menuLines)
		menuW := 0
		for _, line := range menuLines {
			menuW = max(menuW, visibleLen(line))
		}
		cellX = min(max(cellX, 0), max(m.width-menuW, 0))
		cellY = min(max(cellY, 0), max(m.height-menuH, 0))

		frame = overlayAt(frame, menuBox, cellX, cellY)
	}

	return frame
}
