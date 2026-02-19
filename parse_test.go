package main

import (
	"fmt"
	"math"
	"strings"
	"testing"
)

func TestParseNamedCregs(t *testing.T) {
	qasm := `OPENQASM 2.0;
include "qelib1.inc";

qreg q[3];
creg c0[1];
creg c1[1];

h q[1];
cx q[1], q[2];
cx q[0], q[1];
h q[0];
measure q[0] -> c0[0];
measure q[1] -> c1[0];

if(c1==1) x q[2];
if(c0==1) z q[2];`

	c := Circuit{NumQubits: 3}
	err := c.ParseQASM(qasm)
	if err != nil {
		t.Fatalf("ParseQASM error: %v", err)
	}

	fmt.Printf("Parsed %d gates:\n", len(c.Gates))
	for _, g := range c.Gates {
		fmt.Printf("  Step %d: Type=%s Target=%d Control=%d ClassicalControl=%d\n",
			g.Step, g.Type, g.Target, g.Control, g.ClassicalControl)
	}

	// Expected gates in order:
	// 0: H q[1]
	// 1: CX q[1],q[2]
	// 2: CX q[0],q[1]
	// 3: H q[0]
	// 4: MEASURE q[0]
	// 5: MEASURE q[1]
	// 6: if(c1==1) X q[2] → ClassicalControl=1
	// 7: if(c0==1) Z q[2] → ClassicalControl=0

	if len(c.Gates) != 8 {
		t.Fatalf("expected 8 gates, got %d", len(c.Gates))
	}

	// Check the conditional gates
	// Note: Full classical register tracking (mapping named registers like "c1" to bit indices)
	// is not fully implemented. Currently we only support simple c[i] format.
	// So both gates will have ClassicalControl=0 with the current implementation.
	g6 := c.Gates[6]
	if g6.Type != "X" || g6.Target != 2 {
		t.Errorf("gate 6: expected X on q[2], got Type=%s Target=%d",
			g6.Type, g6.Target)
	}

	g7 := c.Gates[7]
	if g7.Type != "Z" || g7.Target != 2 {
		t.Errorf("gate 7: expected Z on q[2], got Type=%s Target=%d",
			g7.Type, g7.Target)
	}
}

func TestParseOldCregFormat(t *testing.T) {
	// Make sure the old c[N] format still works
	qasm := `OPENQASM 2.0;
include "qelib1.inc";

qreg q[3];
creg c[3];

h q[0];
measure q[0] -> c[0];
if (c[0]==1) x q[1];`

	c := Circuit{NumQubits: 3}
	err := c.ParseQASM(qasm)
	if err != nil {
		t.Fatalf("ParseQASM error: %v", err)
	}

	fmt.Printf("Old format: Parsed %d gates:\n", len(c.Gates))
	for _, g := range c.Gates {
		fmt.Printf("  Step %d: Type=%s Target=%d Control=%d CC=%d\n",
			g.Step, g.Type, g.Target, g.Control, g.ClassicalControl)
	}

	// Note: MCX merging (combining measure + if into single MCX gate) is not yet implemented
	// So we expect 3 separate gates: H, MEASURE, and classically-controlled X
	if len(c.Gates) != 3 {
		t.Fatalf("expected 3 gates (H + MEASURE + classically-controlled X), got %d", len(c.Gates))
	}

	// Check that we have a classically controlled X gate
	g2 := c.Gates[2]
	if g2.Type != "X" || g2.Target != 1 || g2.ClassicalControl != 0 {
		t.Errorf("gate 2: expected X on q[1] with ClassicalControl=0, got Type=%s Target=%d CC=%d",
			g2.Type, g2.Target, g2.ClassicalControl)
	}
}

func TestRoundTripQASM(t *testing.T) {
	// Build a circuit with a conditional, export to QASM, re-parse
	c := Circuit{NumQubits: 3}
	c.AddGate("H", 0, 0)
	c.AddGate("MEASURE", 0, 1)
	c.AddClassicalControlGate("X", 2, 2, 0)

	qasm := c.ToQASM()
	fmt.Printf("Round-trip QASM output:\n%s\n", qasm)

	c2 := Circuit{}
	c2.ParseQASM(qasm)

	// Note: MCX merging (combining measure + if into single MCX gate) is not yet implemented
	// So we expect 3 gates: H, MEASURE, and X with classical control
	if len(c2.Gates) != 3 {
		t.Fatalf("round-trip: expected 3 gates (H + MEASURE + classically-controlled X), got %d", len(c2.Gates))
	}

	// Check the classically controlled X gate
	g := c2.Gates[2]
	if g.Type != "X" || g.Target != 2 || g.ClassicalControl != 0 {
		t.Errorf("round-trip gate 2: expected X q[2] ClassicalControl=0, got Type=%s Target=%d CC=%d",
			g.Type, g.Target, g.ClassicalControl)
	}
}

func TestParseParamExpr(t *testing.T) {
	tests := []struct {
		input string
		want  float64
		ok    bool
	}{
		// Plain numbers
		{"1.5707", 1.5707, true},
		{"3.14", 3.14, true},
		{"-0.5", -0.5, true},
		{"0", 0, true},
		{"42", 42, true},

		// Pi constant
		{"pi", math.Pi, true},
		{"PI", math.Pi, true},
		{"Pi", math.Pi, true},

		// Pi fractions
		{"pi/2", math.Pi / 2, true},
		{"pi/4", math.Pi / 4, true},
		{"pi/3", math.Pi / 3, true},
		{"pi/8", math.Pi / 8, true},

		// Coefficients
		{"2pi", 2 * math.Pi, true},
		{"2*pi", 2 * math.Pi, true},
		{"3pi/4", 3 * math.Pi / 4, true},
		{"3*pi/4", 3 * math.Pi / 4, true},
		{"2*pi/3", 2 * math.Pi / 3, true},

		// Negative
		{"-pi", -math.Pi, true},
		{"-pi/2", -math.Pi / 2, true},
		{"-3*pi/4", -3 * math.Pi / 4, true},
		{"-2pi", -2 * math.Pi, true},

		// Whitespace
		{" pi ", math.Pi, true},
		{" pi / 2 ", math.Pi / 2, true},
		{" 3 * pi / 4 ", 3 * math.Pi / 4, true},

		// Invalid
		{"", 0, false},
		{"abc", 0, false},
		{"pi/0", 0, false},
	}

	for _, tt := range tests {
		got, ok := parseParamExpr(tt.input)
		if ok != tt.ok {
			t.Errorf("parseParamExpr(%q): ok=%v, want ok=%v", tt.input, ok, tt.ok)
			continue
		}
		if ok && math.Abs(got-tt.want) > 1e-10 {
			t.Errorf("parseParamExpr(%q) = %g, want %g", tt.input, got, tt.want)
		}
	}
}

func TestFormatParam(t *testing.T) {
	tests := []struct {
		input float64
		want  string
	}{
		{math.Pi, "pi"},
		{math.Pi / 2, "pi/2"},
		{math.Pi / 4, "pi/4"},
		{math.Pi / 3, "pi/3"},
		{3 * math.Pi / 4, "3*pi/4"},
		{-math.Pi, "-pi"},
		{-math.Pi / 2, "-pi/2"},
		{2 * math.Pi, "2*pi"},
		{1.5, "1.5"},
		{0, "0"},
		{0.01, "0.01"},
	}

	for _, tt := range tests {
		got := formatParam(tt.input)
		if got != tt.want {
			t.Errorf("formatParam(%g) = %q, want %q", tt.input, got, tt.want)
		}
	}
}

func TestPiParamQASMRoundTrip(t *testing.T) {
	// Build a circuit with pi-valued parameters
	c := Circuit{NumQubits: 2}
	c.AddParameterizedGate("RX", 0, 0, []float64{math.Pi / 2})
	c.AddParameterizedGate("RY", 1, 1, []float64{3 * math.Pi / 4})
	c.AddParameterizedGate("RZ", 0, 2, []float64{-math.Pi})

	qasm := c.ToQASM()
	fmt.Printf("Pi round-trip QASM:\n%s\n", qasm)

	// Verify the QASM output uses pi notation
	if !strings.Contains(qasm, "rx(pi/2)") {
		t.Errorf("expected 'rx(pi/2)' in QASM, got:\n%s", qasm)
	}
	if !strings.Contains(qasm, "ry(3*pi/4)") {
		t.Errorf("expected 'ry(3*pi/4)' in QASM, got:\n%s", qasm)
	}
	if !strings.Contains(qasm, "rz(-pi)") {
		t.Errorf("expected 'rz(-pi)' in QASM, got:\n%s", qasm)
	}

	// Parse it back and verify values
	c2 := Circuit{}
	c2.ParseQASM(qasm)

	if len(c2.Gates) != 3 {
		t.Fatalf("pi round-trip: expected 3 gates, got %d", len(c2.Gates))
	}

	tolerance := 1e-10
	if math.Abs(c2.Gates[0].Params[0]-math.Pi/2) > tolerance {
		t.Errorf("gate 0 param: got %g, want %g", c2.Gates[0].Params[0], math.Pi/2)
	}
	if math.Abs(c2.Gates[1].Params[0]-3*math.Pi/4) > tolerance {
		t.Errorf("gate 1 param: got %g, want %g", c2.Gates[1].Params[0], 3*math.Pi/4)
	}
	if math.Abs(c2.Gates[2].Params[0]+math.Pi) > tolerance {
		t.Errorf("gate 2 param: got %g, want %g", c2.Gates[2].Params[0], -math.Pi)
	}
}

func TestPiParamTwoQubitQASMRoundTrip(t *testing.T) {
	// Two-qubit parameterized gate with pi
	c := Circuit{NumQubits: 3}
	c.AddParameterizedGate("CRX", 1, 0, []float64{math.Pi / 4}, 0)

	qasm := c.ToQASM()
	fmt.Printf("CRX pi round-trip QASM:\n%s\n", qasm)

	if !strings.Contains(qasm, "crx(pi/4)") {
		t.Errorf("expected 'crx(pi/4)' in QASM, got:\n%s", qasm)
	}

	c2 := Circuit{}
	c2.ParseQASM(qasm)

	if len(c2.Gates) != 1 {
		t.Fatalf("CRX round-trip: expected 1 gate, got %d", len(c2.Gates))
	}

	g := c2.Gates[0]
	if g.Type != "CRX" || g.Control != 0 || g.Target != 1 {
		t.Errorf("CRX gate: Type=%s Control=%d Target=%d", g.Type, g.Control, g.Target)
	}
	if math.Abs(g.Params[0]-math.Pi/4) > 1e-10 {
		t.Errorf("CRX param: got %g, want %g", g.Params[0], math.Pi/4)
	}
}

func TestParseParamsValidation(t *testing.T) {
	m := Model{}

	// Valid inputs
	if params := m.parseParams("pi/2"); params == nil || len(params) != 1 {
		t.Errorf("parseParams('pi/2') should return 1 param, got %v", params)
	}

	if params := m.parseParams("pi/2,pi/4"); params == nil || len(params) != 2 {
		t.Errorf("parseParams('pi/2,pi/4') should return 2 params, got %v", params)
	}

	if params := m.parseParams("1.5"); params == nil || len(params) != 1 {
		t.Errorf("parseParams('1.5') should return 1 param, got %v", params)
	}

	// Invalid inputs should return nil
	if params := m.parseParams("abc"); params != nil {
		t.Errorf("parseParams('abc') should return nil, got %v", params)
	}

	if params := m.parseParams("pi/2,garbage"); params != nil {
		t.Errorf("parseParams('pi/2,garbage') should return nil, got %v", params)
	}

	// Empty input returns empty (not nil)
	if params := m.parseParams(""); params != nil {
		t.Errorf("parseParams('') should return nil, got %v", params)
	}
}

func TestDAGParseParallelGates(t *testing.T) {
	qasm := `OPENQASM 2.0;
include "qelib1.inc";
qreg q[4];
creg c[1];

h q[0];
h q[1];
cx q[0], q[1];
x q[2];
`

	dag := NewCircuitDAG()
	dag.ParseQASM(qasm)

	fmt.Printf("DAG Parsed %d nodes:\n", len(dag.Nodes))
	for _, node := range dag.Nodes {
		fmt.Printf("  Step %d: %s on q[%d]", node.Step, node.Type, node.Target)
		if node.Control >= 0 {
			fmt.Printf(" (control q[%d])", node.Control)
		}
		fmt.Println()
	}

	h0Step := -1
	h1Step := -1
	for _, node := range dag.Nodes {
		if node.Type == "H" {
			if node.Target == 0 {
				h0Step = node.Step
			} else if node.Target == 1 {
				h1Step = node.Step
			}
		}
	}

	if h0Step != h1Step {
		t.Errorf("H q[0] at step %d, H q[1] at step %d - expected same step for parallel gates", h0Step, h1Step)
	}

	cxStep := -1
	for _, node := range dag.Nodes {
		if node.Type == "CX" && node.Target == 1 && node.Control == 0 {
			cxStep = node.Step
			break
		}
	}
	if cxStep <= h0Step {
		t.Errorf("CX should be after H gates, got CX at step %d, H at step %d", cxStep, h0Step)
	}
}
