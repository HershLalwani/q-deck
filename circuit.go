package main

import (
	"fmt"
	"regexp"
	"slices"
	"strconv"
	"strings"
)

// Pre-compiled regexps for QASM parsing.
var (
	singleGateRegex      = regexp.MustCompile(`^(\w+)\s+q\[(\d+)\];?$`)
	singleGateParamRegex = regexp.MustCompile(`^(\w+)\s*\(\s*(` + paramPattern + `(?:\s*,\s*` + paramPattern + `)*)\s*\)\s+q\[(\d+)\];?$`)
	twoQubitRegex        = regexp.MustCompile(`^(\w+)\s+q\[(\d+)\],\s*q\[(\d+)\];?$`)
	twoQubitParamRegex   = regexp.MustCompile(`^(\w+)\s*\(\s*(` + paramPattern + `)\s*\)\s+q\[(\d+)\],\s*q\[(\d+)\];?$`)
	threeQubitRegex      = regexp.MustCompile(`^(\w+)\s+q\[(\d+)\],\s*q\[(\d+)\],\s*q\[(\d+)\];?$`)
	measureRegex         = regexp.MustCompile(`^measure\s+q\[(\d+)\]\s*->\s*(\w+)\[(\d+)\];?$`)
	resetRegex           = regexp.MustCompile(`^reset\s+q\[(\d+)\];?$`)
	ifRegex              = regexp.MustCompile(`^if\s*\(\s*(\w+)(?:\[(\d+)\])?\s*==\s*(\d+)\s*\)\s+(\w+)\s+q\[(\d+)\];?$`)
	ifParamRegex         = regexp.MustCompile(`^if\s*\(\s*(\w+)(?:\[(\d+)\])?\s*==\s*(\d+)\s*\)\s+(\w+)\s*\(\s*(` + paramPattern + `)\s*\)\s+q\[(\d+)\];?$`)
	qregRegex            = regexp.MustCompile(`qreg\s+(\w+)\[(\d+)\]`)
	cregRegex            = regexp.MustCompile(`creg\s+(\w+)\[(\d+)\]`)
	noiseRegex           = regexp.MustCompile(`^//\s*noise\s+(\w+)\s+q\[(\d+)\](?:\s+param=(` + paramPattern + `))?$`)
	barrierRegex         = regexp.MustCompile(`^barrier\s+`)
)

// Gate represents a quantum gate placed on the circuit.
type Gate struct {
	Type             string
	Target           int
	Control          int       // -1 if not a controlled gate
	Controls         []int     // Multiple control qubits (for CCX/Toffoli)
	MeasureSource    int       // -1 if not a measurement-controlled gate
	Step             int       // position in circuit timeline
	Params           []float64 // Parameters for parameterized gates
	IsDagger         bool      // True if gate is dagger (adjoint)
	IsReset          bool      // True if this is a reset operation
	ClassicalControl int       // -1 if not classically controlled, else classical bit index
	IsNoise          bool      // True if this is a noise operation
	NoiseType        string    // Type of noise
}

// Circuit holds the quantum circuit state.
type Circuit struct {
	NumQubits int
	Gates     []Gate
	MaxSteps  int
}

// AddGate appends a gate to the circuit.
func (c *Circuit) AddGate(gateType string, target, step int, control ...int) {
	ctrl := -1
	if len(control) > 0 {
		ctrl = control[0]
	}
	c.Gates = append(c.Gates, Gate{
		Type:             gateType,
		Target:           target,
		Control:          ctrl,
		MeasureSource:    -1,
		Step:             step,
		ClassicalControl: -1,
		IsNoise:          false,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// AddParameterizedGate appends a parameterized gate to the circuit.
func (c *Circuit) AddParameterizedGate(gateType string, target, step int, params []float64, control ...int) {
	ctrl := -1
	if len(control) > 0 {
		ctrl = control[0]
	}
	c.Gates = append(c.Gates, Gate{
		Type:             gateType,
		Target:           target,
		Control:          ctrl,
		MeasureSource:    -1,
		Step:             step,
		Params:           params,
		ClassicalControl: -1,
		IsNoise:          false,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// AddMultiControlGate appends a multi-controlled gate to the circuit.
func (c *Circuit) AddMultiControlGate(gateType string, target, step int, controls []int) {
	c.Gates = append(c.Gates, Gate{
		Type:             gateType,
		Target:           target,
		Control:          -1,
		Controls:         controls,
		MeasureSource:    -1,
		Step:             step,
		ClassicalControl: -1,
		IsNoise:          false,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// AddClassicalControlGate appends a classically-controlled gate to the circuit.
func (c *Circuit) AddClassicalControlGate(gateType string, target, step, cbit int) {
	c.Gates = append(c.Gates, Gate{
		Type:             gateType,
		Target:           target,
		Control:          -1,
		MeasureSource:    -1,
		Step:             step,
		ClassicalControl: cbit,
		IsNoise:          false,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// AddDaggerGate appends a dagger (adjoint) gate to the circuit.
func (c *Circuit) AddDaggerGate(gateType string, target, step int) {
	c.Gates = append(c.Gates, Gate{
		Type:             gateType,
		Target:           target,
		Control:          -1,
		MeasureSource:    -1,
		Step:             step,
		IsDagger:         true,
		ClassicalControl: -1,
		IsNoise:          false,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// AddReset appends a reset gate to the circuit.
func (c *Circuit) AddReset(target, step int) {
	c.Gates = append(c.Gates, Gate{
		Type:             "RESET",
		Target:           target,
		Control:          -1,
		MeasureSource:    -1,
		Step:             step,
		IsReset:          true,
		ClassicalControl: -1,
		IsNoise:          false,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// AddNoise appends a noise operation to the circuit.
func (c *Circuit) AddNoise(target, step int, noiseType string, params ...float64) {
	c.Gates = append(c.Gates, Gate{
		Type:             "NOISE",
		Target:           target,
		Control:          -1,
		MeasureSource:    -1,
		Step:             step,
		Params:           params,
		IsNoise:          true,
		NoiseType:        noiseType,
		ClassicalControl: -1,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// AddMeasureControlGate appends a measurement-controlled gate to the circuit.
func (c *Circuit) AddMeasureControlGate(source, target, step int) {
	c.Gates = append(c.Gates, Gate{
		Type:             "MCX",
		Target:           target,
		Control:          -1,
		MeasureSource:    source,
		Step:             step,
		ClassicalControl: -1,
		IsNoise:          false,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// AddBarrier appends a barrier spanning all qubits at the given step.
func (c *Circuit) AddBarrier(step int) {
	// Remove any existing barrier at this step
	c.Gates = slices.DeleteFunc(c.Gates, func(g Gate) bool {
		return g.Step == step && g.Type == "BARRIER"
	})
	c.Gates = append(c.Gates, Gate{
		Type:             "BARRIER",
		Target:           -1, // spans all qubits
		Control:          -1,
		MeasureSource:    -1,
		Step:             step,
		ClassicalControl: -1,
		IsNoise:          false,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// gateReferences reports whether the gate references the given qubit.
func (g Gate) gateReferences(qubit int) bool {
	if g.Target == qubit || g.Control == qubit || g.MeasureSource == qubit {
		return true
	}
	for _, ctrl := range g.Controls {
		if ctrl == qubit {
			return true
		}
	}
	return false
}

// RemoveGateAt removes any gate at the given step and qubit.
// Also removes barriers at that step since they span all qubits.
func (c *Circuit) RemoveGateAt(step, qubit int) {
	c.Gates = slices.DeleteFunc(c.Gates, func(g Gate) bool {
		if g.Step == step && g.Type == "BARRIER" {
			return true
		}
		return g.Step == step && g.gateReferences(qubit)
	})
}

// RemoveGatesOnQubit removes all gates that reference the given qubit index.
func (c *Circuit) RemoveGatesOnQubit(qubit int) {
	c.Gates = slices.DeleteFunc(c.Gates, func(g Gate) bool {
		return g.gateReferences(qubit)
	})
}

// GetGateAt returns the gate at the given step and qubit, or nil.
func (c *Circuit) GetGateAt(step, qubit int) *Gate {
	for i := range c.Gates {
		g := &c.Gates[i]
		if g.Step == step && g.gateReferences(qubit) {
			return g
		}
	}
	return nil
}

// NumCbits returns the number of classical bits needed (derived from measurements).
// Returns 0 when no measurements exist.
func (c *Circuit) NumCbits() int {
	maxMeasureQubit := -1
	for _, gate := range c.Gates {
		if gate.Type == "MEASURE" {
			maxMeasureQubit = max(maxMeasureQubit, gate.Target)
		}
		if gate.MeasureSource >= 0 {
			maxMeasureQubit = max(maxMeasureQubit, gate.MeasureSource)
		}
	}
	if maxMeasureQubit < 0 {
		return 0
	}
	return maxMeasureQubit + 1
}

// GetMeasureAtStep returns the qubit index being measured at the given step, or -1 if none.
// This is used to determine which classical bit wire receives a value at each step.
func (c *Circuit) GetMeasureAtStep(step int) int {
	for _, g := range c.Gates {
		if g.Step != step {
			continue
		}
		if g.Type == "MEASURE" {
			return g.Target
		}
		if g.MeasureSource >= 0 {
			return g.MeasureSource
		}
	}
	return -1
}

// ToQASM generates QASM 2.0 output from the circuit.
func (c *Circuit) ToQASM() string {
	// Determine actual qubit count and classical bit count based on gates
	maxQubit := -1
	maxMeasureQubit := -1
	maxClassicalControl := -1
	for _, gate := range c.Gates {
		maxQubit = max(maxQubit, gate.Target, gate.Control, gate.MeasureSource)
		for _, ctrl := range gate.Controls {
			maxQubit = max(maxQubit, ctrl)
		}
		if gate.Type == "MEASURE" {
			maxMeasureQubit = max(maxMeasureQubit, gate.Target)
		}
		if gate.MeasureSource >= 0 {
			maxMeasureQubit = max(maxMeasureQubit, gate.MeasureSource)
		}
		if gate.ClassicalControl >= 0 {
			maxClassicalControl = max(maxClassicalControl, gate.ClassicalControl)
		}
	}

	// Use the larger of gate-derived count and visual qubit count
	numQubits := max(maxQubit+1, c.NumQubits, 1)

	// creg must be large enough to hold the highest classical bit index used
	numCbits := max(maxMeasureQubit, maxClassicalControl) + 1
	if numCbits < 1 {
		numCbits = 1
	}

	var sb strings.Builder
	sb.WriteString("OPENQASM 2.0;\n")
	sb.WriteString("include \"qelib1.inc\";\n\n")
	fmt.Fprintf(&sb, "qreg q[%d];\n", numQubits)
	fmt.Fprintf(&sb, "creg c[%d];\n\n", numCbits)

	for step := range c.MaxSteps {
		for _, gate := range c.Gates {
			if gate.Step != step {
				continue
			}
			switch {
			case gate.Type == "BARRIER":
				// Barrier spanning all qubits
				qubits := make([]string, numQubits)
				for q := range numQubits {
					qubits[q] = fmt.Sprintf("q[%d]", q)
				}
				fmt.Fprintf(&sb, "barrier %s;\n", strings.Join(qubits, ", "))
			case gate.IsNoise:
				// Noise operations are comments in QASM since they're not standard
				if len(gate.Params) > 0 {
					fmt.Fprintf(&sb, "// noise %s q[%d] param=%s\n", gate.NoiseType, gate.Target, formatParam(gate.Params[0]))
				} else {
					fmt.Fprintf(&sb, "// noise %s q[%d]\n", gate.NoiseType, gate.Target)
				}
			case gate.IsReset:
				fmt.Fprintf(&sb, "reset q[%d];\n", gate.Target)
			case gate.ClassicalControl >= 0:
				// Classically controlled gate
				if gate.Control >= 0 {
					// Two-qubit classically controlled gate
					fmt.Fprintf(&sb, "if (c[%d]==1) cx q[%d], q[%d];\n", gate.ClassicalControl, gate.Control, gate.Target)
				} else if len(gate.Controls) > 0 {
					// Multi-controlled classically controlled gate
					gateType := strings.ToLower(gate.Type)
					fmt.Fprintf(&sb, "if (c[%d]==1) %s ", gate.ClassicalControl, gateType)
					for i, ctrl := range gate.Controls {
						if i > 0 {
							sb.WriteString(", ")
						}
						fmt.Fprintf(&sb, "q[%d]", ctrl)
					}
					fmt.Fprintf(&sb, ", q[%d];\n", gate.Target)
				} else {
					// Single-qubit classically controlled gate
					gateType := strings.ToLower(gate.Type)
					if len(gate.Params) > 0 {
						fmt.Fprintf(&sb, "if (c[%d]==1) %s(%s) q[%d];\n", gate.ClassicalControl, gateType, formatParam(gate.Params[0]), gate.Target)
					} else if gate.IsDagger {
						fmt.Fprintf(&sb, "if (c[%d]==1) %sdg q[%d];\n", gate.ClassicalControl, gateType, gate.Target)
					} else {
						fmt.Fprintf(&sb, "if (c[%d]==1) %s q[%d];\n", gate.ClassicalControl, gateType, gate.Target)
					}
				}
			case gate.MeasureSource >= 0:
				fmt.Fprintf(&sb, "measure q[%d] -> c[%d];\n", gate.MeasureSource, gate.MeasureSource)
				fmt.Fprintf(&sb, "if (c[%d]==1) x q[%d];\n", gate.MeasureSource, gate.Target)
			case gate.Type == "MEASURE":
				fmt.Fprintf(&sb, "measure q[%d] -> c[%d];\n", gate.Target, gate.Target)
			case len(gate.Controls) > 0:
				// Multi-controlled gates (e.g., Toffoli CCX)
				switch gate.Type {
				case "CCX", "TOFFOLI":
					if len(gate.Controls) >= 2 {
						fmt.Fprintf(&sb, "ccx q[%d], q[%d], q[%d];\n", gate.Controls[0], gate.Controls[1], gate.Target)
					}
				default:
					// Generic multi-controlled gate
					gateType := strings.ToLower(gate.Type)
					fmt.Fprintf(&sb, "%s ", gateType)
					for i, ctrl := range gate.Controls {
						if i > 0 {
							sb.WriteString(", ")
						}
						fmt.Fprintf(&sb, "q[%d]", ctrl)
					}
					fmt.Fprintf(&sb, ", q[%d];\n", gate.Target)
				}
			case gate.Control >= 0:
				switch gate.Type {
				case "CX":
					fmt.Fprintf(&sb, "cx q[%d], q[%d];\n", gate.Control, gate.Target)
				case "CZ":
					fmt.Fprintf(&sb, "cz q[%d], q[%d];\n", gate.Control, gate.Target)
				case "SWAP":
					fmt.Fprintf(&sb, "swap q[%d], q[%d];\n", gate.Control, gate.Target)
				case "CH":
					fmt.Fprintf(&sb, "ch q[%d], q[%d];\n", gate.Control, gate.Target)
				case "CRX":
					if len(gate.Params) > 0 {
						fmt.Fprintf(&sb, "crx(%s) q[%d], q[%d];\n", formatParam(gate.Params[0]), gate.Control, gate.Target)
					}
				case "CRY":
					if len(gate.Params) > 0 {
						fmt.Fprintf(&sb, "cry(%s) q[%d], q[%d];\n", formatParam(gate.Params[0]), gate.Control, gate.Target)
					}
				case "CRZ":
					if len(gate.Params) > 0 {
						fmt.Fprintf(&sb, "crz(%s) q[%d], q[%d];\n", formatParam(gate.Params[0]), gate.Control, gate.Target)
					}
				case "CP", "CU1":
					if len(gate.Params) > 0 {
						fmt.Fprintf(&sb, "cu1(%s) q[%d], q[%d];\n", formatParam(gate.Params[0]), gate.Control, gate.Target)
					}
				default:
					fmt.Fprintf(&sb, "cx q[%d], q[%d];\n", gate.Control, gate.Target)
				}
			default:
				// Single-qubit gates
				gateType := strings.ToLower(gate.Type)
				switch gateType {
				case "rx", "ry", "rz", "p", "u1", "u2", "u3":
					// Parameterized gates
					if len(gate.Params) == 1 {
						fmt.Fprintf(&sb, "%s(%s) q[%d];\n", gateType, formatParam(gate.Params[0]), gate.Target)
					} else if len(gate.Params) == 2 && gateType == "u2" {
						fmt.Fprintf(&sb, "%s(%s, %s) q[%d];\n", gateType, formatParam(gate.Params[0]), formatParam(gate.Params[1]), gate.Target)
					} else if len(gate.Params) == 3 && gateType == "u3" {
						fmt.Fprintf(&sb, "%s(%s, %s, %s) q[%d];\n", gateType, formatParam(gate.Params[0]), formatParam(gate.Params[1]), formatParam(gate.Params[2]), gate.Target)
					}
				case "s", "t":
					if gate.IsDagger {
						fmt.Fprintf(&sb, "%sdg q[%d];\n", gateType, gate.Target)
					} else {
						fmt.Fprintf(&sb, "%s q[%d];\n", gateType, gate.Target)
					}
				case "sx", "sy", "sz":
					// Square root gates
					if gate.IsDagger {
						fmt.Fprintf(&sb, "%sdg q[%d];\n", gateType, gate.Target)
					} else {
						fmt.Fprintf(&sb, "%s q[%d];\n", gateType, gate.Target)
					}
				default:
					fmt.Fprintf(&sb, "%s q[%d];\n", gateType, gate.Target)
				}
			}
		}
	}

	return sb.String()
}

// ParseQASM parses QASM text and rebuilds the circuit from it.
func (c *Circuit) ParseQASM(qasm string) error {
	c.Gates = nil
	c.MaxSteps = 0
	step := 0

	lines := strings.Split(qasm, "\n")

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		if strings.HasPrefix(line, "//") {
			continue
		}
		if strings.HasPrefix(line, "OPENQASM") ||
			strings.HasPrefix(line, "include") {
			continue
		}
		if strings.HasPrefix(line, "qreg") {
			if matches := qregRegex.FindStringSubmatch(line); len(matches) > 1 {
				n, _ := strconv.Atoi(matches[1])
				c.NumQubits = n
			}
			continue
		}
		if strings.HasPrefix(line, "creg") {
			continue
		}
		if strings.HasPrefix(line, "barrier") {
			c.AddBarrier(step)
			step++
			continue
		}

		// Measurement: "measure q[0] -> c[0];"
		if matches := measureRegex.FindStringSubmatch(line); matches != nil {
			source, _ := strconv.Atoi(matches[1])
			c.AddGate("MEASURE", source, step)
			step++
			continue
		}

		// Two-qubit gates: cx, cz, swap
		if matches := twoQubitRegex.FindStringSubmatch(line); matches != nil {
			gateType := strings.ToUpper(matches[1])
			qubit1, _ := strconv.Atoi(matches[2])
			qubit2, _ := strconv.Atoi(matches[3])
			switch gateType {
			case "CX":
				c.AddGate("CX", qubit2, step, qubit1)
			case "CZ":
				c.AddGate("CZ", qubit2, step, qubit1)
			case "SWAP":
				c.AddGate("SWAP", qubit2, step, qubit1)
			default:
				c.AddGate(gateType, qubit2, step, qubit1)
			}
			step++
			continue
		}

		// Single-qubit parameterized gates (RX, RY, RZ, P, U1, U2, U3)
		if matches := singleGateParamRegex.FindStringSubmatch(line); matches != nil {
			gateType := strings.ToUpper(matches[1])
			paramsStr := matches[2]
			target, _ := strconv.Atoi(matches[3])

			var params []float64
			paramStrs := strings.Split(paramsStr, ",")
			for _, pStr := range paramStrs {
				pStr = strings.TrimSpace(pStr)
				if p, ok := parseParamExpr(pStr); ok {
					params = append(params, p)
				}
			}

			c.AddParameterizedGate(gateType, target, step, params)
			step++
			continue
		}

		// Two-qubit parameterized gates (CRX, CRY, CRZ, CU1)
		if matches := twoQubitParamRegex.FindStringSubmatch(line); matches != nil {
			gateType := strings.ToUpper(matches[1])
			param, _ := parseParamExpr(matches[2])
			qubit1, _ := strconv.Atoi(matches[3])
			qubit2, _ := strconv.Atoi(matches[4])
			c.AddParameterizedGate(gateType, qubit2, step, []float64{param}, qubit1)
			step++
			continue
		}

		// Three-qubit gates (Toffoli/CCX)
		if matches := threeQubitRegex.FindStringSubmatch(line); matches != nil {
			gateType := strings.ToUpper(matches[1])
			qubit1, _ := strconv.Atoi(matches[2])
			qubit2, _ := strconv.Atoi(matches[3])
			qubit3, _ := strconv.Atoi(matches[4])
			if gateType == "CCX" || gateType == "TOFFOLI" {
				c.AddMultiControlGate("CCX", qubit3, step, []int{qubit1, qubit2})
			}
			step++
			continue
		}

		// Reset gate
		if matches := resetRegex.FindStringSubmatch(line); matches != nil {
			target, _ := strconv.Atoi(matches[1])
			c.AddReset(target, step)
			step++
			continue
		}

		// Classical control gates
		if matches := ifRegex.FindStringSubmatch(line); matches != nil {
			cbit, _ := strconv.Atoi(matches[1])
			// matches[2] is optional bit index (not used in simple case)
			// matches[3] is the value (should be 1)
			gateType := strings.ToUpper(matches[4])
			target, _ := strconv.Atoi(matches[5])
			c.AddClassicalControlGate(gateType, target, step, cbit)
			step++
			continue
		}

		// Classical control parameterized gates
		if matches := ifParamRegex.FindStringSubmatch(line); matches != nil {
			cbit, _ := strconv.Atoi(matches[1])
			gateType := strings.ToUpper(matches[4])
			param, _ := parseParamExpr(matches[5])
			target, _ := strconv.Atoi(matches[6])
			gate := Gate{
				Type:             gateType,
				Target:           target,
				Control:          -1,
				Step:             step,
				Params:           []float64{param},
				ClassicalControl: cbit,
			}
			c.Gates = append(c.Gates, gate)
			step++
			continue
		}

		// Single-qubit gate (including dagger gates)
		if matches := singleGateRegex.FindStringSubmatch(line); matches != nil {
			gateType := strings.ToUpper(matches[1])
			target, _ := strconv.Atoi(matches[2])

			// Check for dagger gates (sdg, tdg)
			isDagger := false
			if strings.HasSuffix(gateType, "DG") {
				isDagger = true
				gateType = strings.TrimSuffix(gateType, "DG")
			}

			// Check for square root gates with dagger (sxdg, sydg, szdg)
			baseGate := gateType
			if strings.HasPrefix(gateType, "SX") || strings.HasPrefix(gateType, "SY") || strings.HasPrefix(gateType, "SZ") {
				baseGate = gateType
				if strings.HasSuffix(gateType, "DG") {
					isDagger = true
					baseGate = strings.TrimSuffix(gateType, "DG")
				}
			}

			if isDagger {
				c.AddDaggerGate(baseGate, target, step)
			} else {
				c.AddGate(baseGate, target, step)
			}
			step++
			continue
		}
	}

	return nil
}

// getStepWidth returns the cell width needed for the given step.
func (c *Circuit) getStepWidth(step int) int {
	maxW := 3 // minimum cell width
	for _, g := range c.Gates {
		if g.Step != step {
			continue
		}
		// Skip barriers and controls
		if g.Type == "BARRIER" {
			continue
		}
		name := gateDisplayName(g.Type)
		cw := cellWidthForName(name)
		if cw > maxW {
			maxW = cw
		}
	}
	return maxW
}

// getStepWidths returns cell widths for steps in [startStep, startStep+count).
func (c *Circuit) getStepWidths(startStep, count int) []int {
	widths := make([]int, count)
	for i := range count {
		widths[i] = c.getStepWidth(startStep + i)
	}
	return widths
}

// cellInfo describes what occupies a single cell in the circuit grid.
type cellInfo struct {
	gate         *Gate
	isControl    bool
	isTarget     bool
	vertAbove    bool
	vertBelow    bool
	passThrough  bool
	measureBelow bool
	isBarrier    bool
}

// getCellInfo returns rendering information for the cell at (step, qubit).
func (c *Circuit) getCellInfo(step, qubit int) cellInfo {
	var info cellInfo

	gate := c.GetGateAt(step, qubit)
	if gate != nil {
		info.gate = gate
		info.isControl = (gate.Control == qubit)
		info.isTarget = (gate.Target == qubit && gate.Control >= 0)
		if !info.isControl && len(gate.Controls) > 0 {
			for _, ctrl := range gate.Controls {
				if ctrl == qubit {
					info.isControl = true
					break
				}
			}
		}
		if !info.isTarget && gate.Target == qubit && len(gate.Controls) > 0 {
			info.isTarget = true
		}
	}

	// Check for barrier at this step
	for i := range c.Gates {
		if c.Gates[i].Step == step && c.Gates[i].Type == "BARRIER" {
			info.isBarrier = true
			if info.gate == nil {
				info.gate = &c.Gates[i]
			}
			break
		}
	}

	// Vertical connections for two-qubit and measurement-controlled gates
	for _, g := range c.Gates {
		if g.Step != step {
			continue
		}

		var minQ, maxQ int
		switch {
		case len(g.Controls) > 0:
			minQ = g.Target
			maxQ = g.Target
			for _, ctrl := range g.Controls {
				if ctrl < minQ {
					minQ = ctrl
				}
				if ctrl > maxQ {
					maxQ = ctrl
				}
			}
		case g.Control >= 0:
			minQ, maxQ = min(g.Control, g.Target), max(g.Control, g.Target)
		case g.MeasureSource >= 0:
			minQ, maxQ = min(g.MeasureSource, g.Target), max(g.MeasureSource, g.Target)
		default:
			continue
		}

		if qubit >= minQ && qubit <= maxQ {
			if qubit > minQ {
				info.vertAbove = true
			}
			if qubit < maxQ {
				info.vertBelow = true
			}
			if qubit > minQ && qubit < maxQ && info.gate == nil {
				info.passThrough = true
			}
		}
	}

	// Vertical connections for measurement gates going down to classical wires
	for _, g := range c.Gates {
		if g.Step != step {
			continue
		}
		measuredQubit := -1
		if g.Type == "MEASURE" {
			measuredQubit = g.Target
		} else if g.MeasureSource >= 0 {
			measuredQubit = g.MeasureSource
		}
		if measuredQubit >= 0 && qubit > measuredQubit {
			info.measureBelow = true
		}
	}

	return info
}

// cellWidthForName returns the cell width needed for a gate name.
func cellWidthForName(name string) int {
	// Minimum width of 3, plus extra for longer names
	if len(name) <= 1 {
		return 3
	}
	return len(name) + 2
}
