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
	focusInputParam
	focusSelectControls
	focusEditGate
	focusEditParam
	focusEditTarget
	focusEditControl
)

// Model represents the TUI application state.
type Model struct {
	dag           *CircuitDAG // DAG is the single source of truth
	circuit       Circuit     // Circuit view derived from DAG
	cursorQubit   int
	cursorStep    int
	viewStartStep int // First step currently visible in the view
	width         int
	height        int
	qasmEditor    textarea.Model
	focus         focus
	lastQASM      string
	statusMsg     string // transient status message (e.g. save confirmation)

	// Menu state
	menuCat  int
	menuItem int

	// Target-selection state (for multi-qubit gates)
	pendingGate   string
	targetQubit   int
	paramInput    string
	controlQubits []int

	// Edit gate state
	editGate       *Gate // pointer to the gate being edited
	editMenuIdx    int   // selected option in edit menu
	editOrigStep   int   // step of the gate being edited
	editControlIdx int   // which control index is being edited (-1 for single Control field)
}

func initialModel() Model {
	ta := textarea.New()
	ta.Placeholder = "Edit QASM here..."
	ta.SetWidth(40)
	ta.SetHeight(20)
	ta.ShowLineNumbers = true
	ta.KeyMap.InsertNewline.SetEnabled(true)

	dag := NewCircuitDAG()
	dag.NumQubits = 4

	m := Model{
		dag:           dag,
		circuit:       *dag.ToCircuit(),
		qasmEditor:    ta,
		focus:         focusCircuit,
		viewStartStep: 0,
	}

	m.syncFromDAG()
	return m
}

func (m *Model) syncFromDAG() {
	// Update circuit view from DAG
	m.circuit = *m.dag.ToCircuit()

	// Update QASM view from DAG
	qasm := m.dag.ToQASM()
	m.qasmEditor.SetValue(qasm)
	m.lastQASM = qasm
}

func (m *Model) parseQASMInput() {
	qasm := m.qasmEditor.Value()
	if qasm != m.lastQASM {
		// Parse QASM into DAG first (DAG is source of truth)
		dag := NewCircuitDAG()
		dag.ParseQASM(qasm)
		m.dag = dag

		// Update circuit view from DAG
		m.circuit = *m.dag.ToCircuit()
		m.lastQASM = qasm
	}
}

// placeGate places a gate on the circuit at the cursor position.
// targetQ is the target qubit for multi-qubit gates (-1 for single-qubit).
// Returns true if placement succeeded, false if blocked by conflict.
func (m *Model) placeGate(gateType string, targetQ int) bool {
	var qubitsNeeded []int
	switch gateType {
	case "CX", "CZ", "SWAP", "CH", "CRX", "CRY", "CRZ", "CU1":
		qubitsNeeded = []int{m.cursorQubit, targetQ}
	case "CCX":
		qubitsNeeded = []int{m.cursorQubit, targetQ}
		qubitsNeeded = append(qubitsNeeded, m.controlQubits...)
	case "MCX":
		qubitsNeeded = []int{m.cursorQubit, targetQ}
	case "BARRIER":
		qubitsNeeded = nil
	default:
		qubitsNeeded = []int{m.cursorQubit}
	}

	if len(qubitsNeeded) > 0 && !m.dag.CanPlaceGateAt(m.cursorStep, qubitsNeeded) {
		m.statusMsg = "Cannot place: qubit already used by another gate at this step"
		m.paramInput = ""
		m.controlQubits = nil
		m.pendingGate = ""
		return false
	}

	for _, q := range qubitsNeeded {
		m.dag.RemoveNodeAt(m.cursorStep, q)
	}

	switch gateType {
	case "CX", "CZ", "SWAP", "CH", "CRX", "CRY", "CRZ", "CU1":
		// Parameterized controlled gates
		if len(m.paramInput) > 0 {
			params := m.parseParams(m.paramInput)
			if len(params) > 0 {
				m.dag.AddParameterizedGate(gateType, targetQ, m.cursorStep, params, m.cursorQubit)
			} else {
				m.dag.AddGate(gateType, targetQ, m.cursorStep, m.cursorQubit)
			}
		} else {
			m.dag.AddGate(gateType, targetQ, m.cursorStep, m.cursorQubit)
		}
	case "CCX":
		// Toffoli gate - needs two controls and one target
		controls := []int{m.cursorQubit}
		if len(m.controlQubits) > 0 {
			controls = append(controls, m.controlQubits...)
			for _, cq := range m.controlQubits {
				m.dag.RemoveNodeAt(m.cursorStep, cq)
			}
		}
		m.dag.AddMultiControlGate("CCX", targetQ, m.cursorStep, controls)
	case "MCX":
		m.dag.AddMeasureControlGate(m.cursorQubit, targetQ, m.cursorStep)
	case "MEASURE":
		m.dag.AddGate("MEASURE", m.cursorQubit, m.cursorStep)
	case "BARRIER":
		m.dag.AddBarrier(m.cursorStep)
	case "RESET":
		m.dag.AddReset(m.cursorQubit, m.cursorStep)
	case "RX", "RY", "RZ", "P", "U1":
		// Single-qubit parameterized gates
		if len(m.paramInput) > 0 {
			params := m.parseParams(m.paramInput)
			m.dag.AddParameterizedGate(gateType, m.cursorQubit, m.cursorStep, params)
		} else {
			m.dag.AddParameterizedGate(gateType, m.cursorQubit, m.cursorStep, []float64{0.0})
		}
	case "U2":
		params := m.parseParams(m.paramInput)
		if len(params) < 2 {
			params = []float64{0.0, 0.0}
		}
		m.dag.AddParameterizedGate(gateType, m.cursorQubit, m.cursorStep, params[:2])
	case "U3":
		params := m.parseParams(m.paramInput)
		if len(params) < 3 {
			params = []float64{0.0, 0.0, 0.0}
		}
		m.dag.AddParameterizedGate(gateType, m.cursorQubit, m.cursorStep, params[:3])
	case "SDG", "TDG":
		// Dagger gates
		baseType := strings.TrimSuffix(gateType, "DG")
		m.dag.AddDaggerGate(baseType, m.cursorQubit, m.cursorStep)
	case "NOISE_DEPOL", "NOISE_AMP", "NOISE_PHASE":
		// Noise operations
		noiseType := strings.TrimPrefix(gateType, "NOISE_")
		noiseType = strings.ToLower(noiseType)
		if noiseType == "depol" {
			noiseType = "depolarizing"
		} else if noiseType == "amp" {
			noiseType = "amplitude_damping"
		} else if noiseType == "phase" {
			noiseType = "phase_damping"
		}
		params := m.parseParams(m.paramInput)
		if len(params) == 0 {
			params = []float64{0.01}
		}
		m.dag.AddNoise(m.cursorQubit, m.cursorStep, noiseType, params...)
	default:
		// Single-qubit gate
		m.dag.AddGate(gateType, m.cursorQubit, m.cursorStep)
	}

	// Clear temporary state
	m.paramInput = ""
	m.controlQubits = nil
	m.pendingGate = ""

	m.cursorStep++
	m.circuit.MaxSteps = max(m.circuit.MaxSteps, m.cursorStep)
	m.syncFromDAG()
	return true
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
		m.statusMsg = ""

		if key == "ctrl+c" {
			return m, tea.Quit
		}

		switch m.focus {
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
				m.viewStartStep = 0
				m.syncFromDAG()
			case "ctrl+s":
				qasm := m.dag.ToQASM()
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
				if m.cursorQubit < m.dag.NumQubits-1 {
					m.cursorQubit++
				}
			case "left", "h":
				if m.cursorStep > 0 {
					m.cursorStep--
					if m.cursorStep < m.viewStartStep {
						m.viewStartStep = m.cursorStep
					}
				}
			case "right", "l":
				m.cursorStep++
				m.circuit.MaxSteps = max(m.circuit.MaxSteps, m.cursorStep)
			case "+", "=":
				m.dag.NumQubits++
				m.syncFromDAG()
			case "-":
				if m.dag.NumQubits > 1 {
					m.dag.NumQubits--
					m.cursorQubit = min(m.cursorQubit, m.dag.NumQubits-1)
					m.dag.RemoveNodesOnQubit(m.dag.NumQubits)
					m.syncFromDAG()
				}
			case "a":
				m.focus = focusMenu
				m.menuCat = 0
				m.menuItem = 0
			case "backspace", "delete":
				m.dag.RemoveNodeAt(m.cursorStep, m.cursorQubit)
				m.syncFromDAG()
			case "e":
				node := m.dag.GetNodeAt(m.cursorStep, m.cursorQubit)
				if node != nil {
					// Convert DAGNode to Gate for editing
					gate := Gate{
						Type:             node.Type,
						Target:           node.Target,
						Control:          node.Control,
						Controls:         node.Controls,
						MeasureSource:    node.MeasureSource,
						Step:             node.Step,
						Params:           node.Params,
						IsDagger:         node.IsDagger,
						IsReset:          node.IsReset,
						ClassicalControl: node.ClassicalControl,
						IsNoise:          node.IsNoise,
						NoiseType:        node.NoiseType,
					}
					m.editGate = &gate
					m.editMenuIdx = 0
					m.editOrigStep = m.cursorStep
					m.focus = focusEditGate
				}
			}

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
				m.pendingGate = item.gateType

				if isParameterizedGate(item.gateType) {
					m.paramInput = ""
					m.focus = focusInputParam
					break
				}

				if item.gateType == "CCX" {
					if m.dag.NumQubits < 3 {
						// Not enough qubits for Toffoli
						break
					}
					m.controlQubits = nil
					m.focus = focusSelectControls
					m.targetQubit = m.cursorQubit + 1
					if m.targetQubit >= m.dag.NumQubits {
						m.targetQubit = m.cursorQubit - 1
					}
					break
				}

				if item.needsTarget {
					if m.dag.NumQubits < 2 {
						break
					}
					m.focus = focusSelectTarget
					m.targetQubit = m.cursorQubit + 1
					if m.targetQubit >= m.dag.NumQubits {
						m.targetQubit = m.cursorQubit - 1
					}
				} else {
					if m.placeGate(item.gateType, -1) {
						m.focus = focusCircuit
					}
				}
			}

		case focusSelectTarget:
			switch key {
			case "esc":
				m.focus = focusCircuit
				m.paramInput = ""
				m.controlQubits = nil
				m.pendingGate = ""
			case "up", "k":
				for next := m.targetQubit - 1; next >= 0; next-- {
					if next != m.cursorQubit && !slicesContains(m.controlQubits, next) {
						m.targetQubit = next
						break
					}
				}
			case "down", "j":
				for next := m.targetQubit + 1; next < m.dag.NumQubits; next++ {
					if next != m.cursorQubit && !slicesContains(m.controlQubits, next) {
						m.targetQubit = next
						break
					}
				}
			case "enter":
				if m.placeGate(m.pendingGate, m.targetQubit) {
					m.focus = focusCircuit
				}
			}

		case focusSelectControls:
			switch key {
			case "esc":
				m.focus = focusCircuit
				m.paramInput = ""
				m.controlQubits = nil
				m.pendingGate = ""
			case "up", "k":
				for next := m.targetQubit - 1; next >= 0; next-- {
					if next != m.cursorQubit {
						m.targetQubit = next
						break
					}
				}
			case "down", "j":
				for next := m.targetQubit + 1; next < m.dag.NumQubits; next++ {
					if next != m.cursorQubit {
						m.targetQubit = next
						break
					}
				}
			case "enter":
				m.controlQubits = append(m.controlQubits, m.targetQubit)
				m.focus = focusSelectTarget
				for q := 0; q < m.dag.NumQubits; q++ {
					if q != m.cursorQubit && !slicesContains(m.controlQubits, q) {
						m.targetQubit = q
						break
					}
				}
			}

		case focusEditGate:
			if m.editGate == nil {
				m.focus = focusCircuit
				break
			}
			editOptions := m.getEditOptions()
			switch key {
			case "esc":
				m.focus = focusCircuit
				m.editGate = nil
			case "up", "k":
				if m.editMenuIdx > 0 {
					m.editMenuIdx--
				}
			case "down", "j":
				if m.editMenuIdx < len(editOptions)-1 {
					m.editMenuIdx++
				}
			case "enter":
				if m.editMenuIdx < len(editOptions) {
					opt := editOptions[m.editMenuIdx]
					switch opt.action {
					case "edit_param":
						m.paramInput = ""
						m.focus = focusEditParam
					case "edit_target":
						m.targetQubit = m.editGate.Target
						m.focus = focusEditTarget
					case "edit_control":
						m.editControlIdx = opt.ctrlIdx
						if opt.ctrlIdx == -1 {
							m.targetQubit = m.editGate.Control
						} else {
							m.targetQubit = m.editGate.Controls[opt.ctrlIdx]
						}
						m.focus = focusEditControl
					case "delete":
						m.dag.RemoveNodeAt(m.editOrigStep, m.editGate.Target)
						m.editGate = nil
						m.focus = focusCircuit
						m.syncFromDAG()
					}
				}
			}

		case focusEditParam:
			switch key {
			case "esc":
				m.paramInput = ""
				m.focus = focusEditGate
			case "backspace":
				if len(m.paramInput) > 0 {
					m.paramInput = m.paramInput[:len(m.paramInput)-1]
				}
			case "enter":
				if m.editGate != nil {
					params := m.parseParams(m.paramInput)
					if m.paramInput != "" && params == nil {
						m.statusMsg = "Invalid parameter — use numbers or pi expressions (e.g. pi/2, 3*pi/4)"
						break
					}
					if len(params) > 0 {
						m.editGate.Params = params
					}
					m.syncFromDAG()
				}
				m.paramInput = ""
				m.focus = focusEditGate
			default:
				if len(key) == 1 {
					ch := key[0]
					if (ch >= '0' && ch <= '9') || ch == '.' || ch == ',' || ch == '-' || ch == 'e' || ch == 'E' || ch == '+' ||
						ch == 'p' || ch == 'i' || ch == '*' || ch == '/' {
						m.paramInput += key
					}
				}
			}

		case focusEditTarget:
			switch key {
			case "esc":
				m.focus = focusEditGate
			case "up", "k":
				for next := m.targetQubit - 1; next >= 0; next-- {
					if next != m.editGate.Control && !slicesContains(m.editGate.Controls, next) {
						m.targetQubit = next
						break
					}
				}
			case "down", "j":
				for next := m.targetQubit + 1; next < m.dag.NumQubits; next++ {
					if next != m.editGate.Control && !slicesContains(m.editGate.Controls, next) {
						m.targetQubit = next
						break
					}
				}
			case "enter":
				if m.editGate != nil {
					m.editGate.Target = m.targetQubit
					m.syncFromDAG()
				}
				m.focus = focusEditGate
			}

		case focusEditControl:
			unavailable := map[int]bool{m.editGate.Target: true}
			if m.editControlIdx == -1 {
			} else {
				for ci, cq := range m.editGate.Controls {
					if ci != m.editControlIdx {
						unavailable[cq] = true
					}
				}
			}
			switch key {
			case "esc":
				m.focus = focusEditGate
			case "up", "k":
				for next := m.targetQubit - 1; next >= 0; next-- {
					if !unavailable[next] {
						m.targetQubit = next
						break
					}
				}
			case "down", "j":
				for next := m.targetQubit + 1; next < m.dag.NumQubits; next++ {
					if !unavailable[next] {
						m.targetQubit = next
						break
					}
				}
			case "enter":
				if m.editGate != nil {
					if m.editControlIdx == -1 {
						m.editGate.Control = m.targetQubit
					} else if m.editControlIdx < len(m.editGate.Controls) {
						m.editGate.Controls[m.editControlIdx] = m.targetQubit
					}
					m.syncFromDAG()
				}
				m.focus = focusEditGate
			}

		case focusInputParam:
			switch key {
			case "esc":
				m.focus = focusCircuit
				m.paramInput = ""
				m.pendingGate = ""
			case "backspace":
				if len(m.paramInput) > 0 {
					m.paramInput = m.paramInput[:len(m.paramInput)-1]
				}
			case "enter":
				params := m.parseParams(m.paramInput)
				if m.paramInput != "" && params == nil {
					m.statusMsg = "Invalid parameter — use numbers or pi expressions (e.g. pi/2, 3*pi/4)"
					break
				}
				item := gateMenu[m.menuCat].items[m.menuItem]
				if item.needsTarget {
					if m.dag.NumQubits < 2 {
						break
					}
					m.focus = focusSelectTarget
					m.targetQubit = m.cursorQubit + 1
					if m.targetQubit >= m.dag.NumQubits {
						m.targetQubit = m.cursorQubit - 1
					}
				} else {
					if m.placeGate(m.pendingGate, -1) {
						m.focus = focusCircuit
					}
				}
			default:
				if len(key) == 1 {
					ch := key[0]
					if (ch >= '0' && ch <= '9') || ch == '.' || ch == ',' || ch == '-' || ch == 'e' || ch == 'E' || ch == '+' ||
						ch == 'p' || ch == 'i' || ch == '*' || ch == '/' {
						m.paramInput += key
					}
				}
			}

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

// Helper function
func slicesContains(slice []int, val int) bool {
	for _, item := range slice {
		if item == val {
			return true
		}
	}
	return false
}

// editOption represents an option in the edit gate menu.
type editOption struct {
	label   string
	action  string
	ctrlIdx int
}

// getEditOptions returns available edit options for the current gate.
func (m *Model) getEditOptions() []editOption {
	if m.editGate == nil {
		return nil
	}
	var opts []editOption

	if len(m.editGate.Params) > 0 || isParameterizedGate(m.editGate.Type) {
		paramStr := ""
		for i, p := range m.editGate.Params {
			if i > 0 {
				paramStr += ", "
			}
			paramStr += formatParam(p)
		}
		if paramStr == "" {
			paramStr = "none"
		}
		opts = append(opts, editOption{
			label:  fmt.Sprintf("Parameters: %s", paramStr),
			action: "edit_param",
		})
	}

	opts = append(opts, editOption{
		label:  fmt.Sprintf("Target: q[%d]", m.editGate.Target),
		action: "edit_target",
	})

	if m.editGate.Control >= 0 {
		opts = append(opts, editOption{
			label:   fmt.Sprintf("Control: q[%d]", m.editGate.Control),
			action:  "edit_control",
			ctrlIdx: -1,
		})
	}
	for i, ctrl := range m.editGate.Controls {
		opts = append(opts, editOption{
			label:   fmt.Sprintf("Control %d: q[%d]", i+1, ctrl),
			action:  "edit_control",
			ctrlIdx: i,
		})
	}

	opts = append(opts, editOption{
		label:  "Delete gate",
		action: "delete",
	})

	return opts
}

// View renders the UI.
func (m Model) View() string {
	if m.width == 0 {
		return "Loading..."
	}

	qasmWidth := m.width / 3
	circuitWidth := m.width - qasmWidth - 4
	controlsHeight := 6
	circuitHeight := max(m.height-controlsHeight-2, 6)

	circuitPanel := m.renderCircuitPanel(circuitWidth, circuitHeight)
	qasmPanel := m.renderQASMPanel(qasmWidth, circuitHeight)
	controlsPanel := m.renderControlsPanel(m.width-4, controlsHeight-2)

	topRow := lipgloss.JoinHorizontal(lipgloss.Top, circuitPanel, qasmPanel)
	frame := lipgloss.JoinVertical(lipgloss.Left, topRow, controlsPanel)

	// Render menu overlay when in menu mode
	if m.focus == focusMenu {
		menuBox := m.renderMenu()
		// Position menu near cursor or center it
		frame = overlayAt(frame, menuBox, 2, 2)
	}

	// Render parameter input overlay
	if m.focus == focusInputParam {
		paramBox := m.renderParamInput()
		frame = overlayAt(frame, paramBox, 2, 2)
	}

	// Render edit gate menu overlay
	if m.focus == focusEditGate {
		editBox := m.renderEditGateMenu()
		frame = overlayAt(frame, editBox, 2, 2)
	}

	return frame
}

// renderParamInput renders parameter input overlay.
func (m Model) renderParamInput() string {
	var sb strings.Builder
	sb.WriteString(titleStyle.Render("Enter Parameter"))
	sb.WriteString("\n\n")
	sb.WriteString(fmt.Sprintf("Value: %s_", m.paramInput))
	sb.WriteString("\n\n")
	sb.WriteString(dimStyle.Render("Examples: pi/2, 3*pi/4, 1.57"))
	return menuBorderStyle.Render(sb.String())
}

// renderEditGateMenu renders the edit gate menu overlay.
func (m Model) renderEditGateMenu() string {
	var sb strings.Builder
	sb.WriteString(titleStyle.Render("Edit Gate"))
	sb.WriteString("\n\n")
	opts := m.getEditOptions()
	for i, opt := range opts {
		if i == m.editMenuIdx {
			sb.WriteString(menuSelectedStyle.Render(fmt.Sprintf("▸ %s", opt.label)))
		} else {
			sb.WriteString(fmt.Sprintf("  %s", opt.label))
		}
		sb.WriteString("\n")
	}
	sb.WriteString("\n")
	sb.WriteString(dimStyle.Render("↑↓ Select  ⏎ Ok  Esc ✕"))
	return menuBorderStyle.Render(sb.String())
}
