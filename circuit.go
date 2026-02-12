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
	singleGateRegex = regexp.MustCompile(`^(\w+)\s+q\[(\d+)\];?$`)
	twoQubitRegex   = regexp.MustCompile(`^(\w+)\s+q\[(\d+)\],\s*q\[(\d+)\];?$`)
	measureRegex    = regexp.MustCompile(`^measure\s+q\[(\d+)\]\s*->\s*c\[(\d+)\];?$`)
	ifRegex         = regexp.MustCompile(`^if\s*\(\s*c\[(\d+)\]\s*==\s*1\s*\)\s+x\s+q\[(\d+)\];?$`)
	qregRegex       = regexp.MustCompile(`qreg\s+q\[(\d+)\]`)
)

// Gate represents a quantum gate placed on the circuit.
type Gate struct {
	Type          string
	Target        int
	Control       int // -1 if not a controlled gate
	MeasureSource int // -1 if not a measurement-controlled gate
	Step          int // position in circuit timeline
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
		Type:          gateType,
		Target:        target,
		Control:       ctrl,
		MeasureSource: -1,
		Step:          step,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// AddMeasureControlGate appends a measurement-controlled gate to the circuit.
func (c *Circuit) AddMeasureControlGate(source, target, step int) {
	c.Gates = append(c.Gates, Gate{
		Type:          "MCX",
		Target:        target,
		Control:       -1,
		MeasureSource: source,
		Step:          step,
	})
	if step >= c.MaxSteps {
		c.MaxSteps = step + 1
	}
}

// gateReferences reports whether the gate references the given qubit.
func (g Gate) gateReferences(qubit int) bool {
	return g.Target == qubit || g.Control == qubit || g.MeasureSource == qubit
}

// RemoveGateAt removes any gate at the given step and qubit.
func (c *Circuit) RemoveGateAt(step, qubit int) {
	c.Gates = slices.DeleteFunc(c.Gates, func(g Gate) bool {
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
	for _, gate := range c.Gates {
		maxQubit = max(maxQubit, gate.Target, gate.Control, gate.MeasureSource)
		if gate.Type == "MEASURE" {
			maxMeasureQubit = max(maxMeasureQubit, gate.Target)
		}
		if gate.MeasureSource >= 0 {
			maxMeasureQubit = max(maxMeasureQubit, gate.MeasureSource)
		}
	}

	// Use the larger of gate-derived count and visual qubit count
	numQubits := max(maxQubit+1, c.NumQubits, 1)

	// creg must be large enough to hold the highest classical bit index used
	numCbits := max(maxMeasureQubit+1, 1)

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
			case gate.MeasureSource >= 0:
				fmt.Fprintf(&sb, "measure q[%d] -> c[%d];\n", gate.MeasureSource, gate.MeasureSource)
				fmt.Fprintf(&sb, "if (c[%d]==1) x q[%d];\n", gate.MeasureSource, gate.Target)
			case gate.Type == "MEASURE":
				fmt.Fprintf(&sb, "measure q[%d] -> c[%d];\n", gate.Target, gate.Target)
			case gate.Control >= 0:
				switch gate.Type {
				case "CX":
					fmt.Fprintf(&sb, "cx q[%d], q[%d];\n", gate.Control, gate.Target)
				case "CZ":
					fmt.Fprintf(&sb, "cz q[%d], q[%d];\n", gate.Control, gate.Target)
				case "SWAP":
					fmt.Fprintf(&sb, "swap q[%d], q[%d];\n", gate.Control, gate.Target)
				default:
					fmt.Fprintf(&sb, "cx q[%d], q[%d];\n", gate.Control, gate.Target)
				}
			default:
				fmt.Fprintf(&sb, "%s q[%d];\n", strings.ToLower(gate.Type), gate.Target)
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

	for i := range len(lines) {
		line := strings.TrimSpace(lines[i])
		if line == "" || strings.HasPrefix(line, "//") {
			continue
		}
		if strings.HasPrefix(line, "OPENQASM") ||
			strings.HasPrefix(line, "include") ||
			strings.HasPrefix(line, "qreg") ||
			strings.HasPrefix(line, "creg") {
			if strings.HasPrefix(line, "qreg") {
				if matches := qregRegex.FindStringSubmatch(line); len(matches) > 1 {
					n, _ := strconv.Atoi(matches[1])
					c.NumQubits = n
				}
			}
			continue
		}

		// Measurement-controlled gate (measure + if)
		if matches := measureRegex.FindStringSubmatch(line); matches != nil {
			source, _ := strconv.Atoi(matches[1])
			if i+1 < len(lines) {
				nextLine := strings.TrimSpace(lines[i+1])
				if ifMatches := ifRegex.FindStringSubmatch(nextLine); ifMatches != nil {
					condReg, _ := strconv.Atoi(ifMatches[1])
					target, _ := strconv.Atoi(ifMatches[2])
					if condReg == source {
						c.AddMeasureControlGate(source, target, step)
						step++
						i++
						continue
					}
				}
			}
			// Standalone measure
			c.AddGate("MEASURE", source, step)
			step++
			continue
		}

		// Conditional without preceding measure (skip)
		if ifRegex.MatchString(line) {
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

		// Single-qubit gate
		if matches := singleGateRegex.FindStringSubmatch(line); matches != nil {
			gateType := strings.ToUpper(matches[1])
			target, _ := strconv.Atoi(matches[2])
			c.AddGate(gateType, target, step)
			step++
		}
	}

	return nil
}

// cellInfo describes what occupies a single cell in the circuit grid.
type cellInfo struct {
	gate         *Gate
	isControl    bool
	isTarget     bool
	vertAbove    bool
	vertBelow    bool
	passThrough  bool
	measureBelow bool // vertical connection from a MEASURE gate going down to classical wire
}

// getCellInfo returns rendering information for the cell at (step, qubit).
func (c *Circuit) getCellInfo(step, qubit int) cellInfo {
	var info cellInfo

	gate := c.GetGateAt(step, qubit)
	if gate != nil {
		info.gate = gate
		info.isControl = (gate.Control == qubit)
		info.isTarget = (gate.Target == qubit && gate.Control >= 0)
	}

	// Vertical connections for two-qubit and measurement-controlled gates
	for _, g := range c.Gates {
		if g.Step != step {
			continue
		}

		var minQ, maxQ int
		switch {
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

	// Vertical connections for measurement gates going down to classical wires.
	// Any qubit below a MEASURE gate at the same step gets a vertical line through it.
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
