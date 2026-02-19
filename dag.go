package main

import (
	"fmt"
	"slices"
	"strconv"
	"strings"
)

// DAGNode represents a gate in the circuit as a node in a DAG.
// Dependencies represent ordering constraints - a gate cannot execute before
// the gates that affect the same qubits in prior steps.
type DAGNode struct {
	ID               string    // Unique identifier for this node
	Type             string    // Gate type: "H", "X", "CX", "RX", etc.
	Target           int       // Target qubit index
	Control          int       // Control qubit for 2-qubit gates (-1 if none)
	Controls         []int     // Multiple control qubits (for CCX/Toffoli)
	MeasureSource    int       // Source qubit for measurement-controlled gates (-1 if none)
	Step             int       // Position in circuit timeline (for ordering only)
	Params           []float64 // Parameters for rotation gates
	IsDagger         bool      // True for adjoint gates
	IsReset          bool      // True for reset operation
	ClassicalControl int       // Classical bit index for conditional gates (-1 if none)
	IsNoise          bool      // True for noise operations
	NoiseType        string    // Type of noise
	Dependencies     []string  // IDs of nodes that must execute before this one
}

// CircuitDAG represents a quantum circuit as a Directed Acyclic Graph.
// It serves as the single source of truth that both Circuit and QASM views derive from.
type CircuitDAG struct {
	Nodes     map[string]*DAGNode // All nodes by ID
	NumQubits int                 // Number of qubits in the circuit
	NumCbits  int                 // Number of classical bits
	rootNodes []string            // Node IDs with no dependencies (for topological sort)
}

// NewCircuitDAG creates a new empty CircuitDAG.
func NewCircuitDAG() *CircuitDAG {
	return &CircuitDAG{
		Nodes:     make(map[string]*DAGNode),
		NumQubits: 0,
		NumCbits:  0,
		rootNodes: []string{},
	}
}

// generateNodeID creates a unique ID for a node based on its properties.
func generateNodeID(gateType string, target, step int) string {
	return fmt.Sprintf("%s_q%d_s%d", gateType, target, step)
}

// AddNode adds a new gate node to the DAG.
func (dag *CircuitDAG) AddNode(node *DAGNode) {
	if node.ID == "" {
		node.ID = generateNodeID(node.Type, node.Target, node.Step)
	}
	dag.Nodes[node.ID] = node
	dag.updateRootNodes()

	// Update qubit count
	maxQubit := node.Target
	if node.Control > maxQubit {
		maxQubit = node.Control
	}
	for _, ctrl := range node.Controls {
		if ctrl > maxQubit {
			maxQubit = ctrl
		}
	}
	if node.MeasureSource > maxQubit {
		maxQubit = node.MeasureSource
	}
	if maxQubit+1 > dag.NumQubits {
		dag.NumQubits = maxQubit + 1
	}

	// Update classical bit count
	if node.ClassicalControl >= dag.NumCbits {
		dag.NumCbits = node.ClassicalControl + 1
	}
	if node.Type == "MEASURE" && node.Target+1 > dag.NumCbits {
		dag.NumCbits = node.Target + 1
	}
	if node.MeasureSource >= 0 && node.MeasureSource+1 > dag.NumCbits {
		dag.NumCbits = node.MeasureSource + 1
	}
}

// RemoveNode removes a node from the DAG and updates dependencies.
func (dag *CircuitDAG) RemoveNode(nodeID string) {
	delete(dag.Nodes, nodeID)

	// Remove this node from all dependency lists
	for _, node := range dag.Nodes {
		node.Dependencies = slices.DeleteFunc(node.Dependencies, func(dep string) bool {
			return dep == nodeID
		})
	}

	dag.updateRootNodes()
}

// updateRootNodes recalculates the list of root nodes (nodes with no dependencies).
func (dag *CircuitDAG) updateRootNodes() {
	dag.rootNodes = []string{}
	for id, node := range dag.Nodes {
		if len(node.Dependencies) == 0 {
			dag.rootNodes = append(dag.rootNodes, id)
		}
	}
}

// TopologicalSort returns nodes in topological order (respecting dependencies).
func (dag *CircuitDAG) TopologicalSort() []*DAGNode {
	visited := make(map[string]bool)
	result := make([]*DAGNode, 0, len(dag.Nodes))

	var visit func(nodeID string)
	visit = func(nodeID string) {
		if visited[nodeID] {
			return
		}
		visited[nodeID] = true

		node := dag.Nodes[nodeID]
		for _, depID := range node.Dependencies {
			visit(depID)
		}
		result = append(result, node)
	}

	// Visit all root nodes first
	for _, rootID := range dag.rootNodes {
		visit(rootID)
	}

	// Visit any remaining unvisited nodes
	for id := range dag.Nodes {
		visit(id)
	}

	return result
}

// GetNodesAtStep returns all nodes at a specific step.
func (dag *CircuitDAG) GetNodesAtStep(step int) []*DAGNode {
	var result []*DAGNode
	for _, node := range dag.Nodes {
		if node.Step == step {
			result = append(result, node)
		}
	}
	return result
}

// GetNodesOnQubit returns all nodes that reference a specific qubit.
func (dag *CircuitDAG) GetNodesOnQubit(qubit int) []*DAGNode {
	var result []*DAGNode
	for _, node := range dag.Nodes {
		if node.Target == qubit || node.Control == qubit || node.MeasureSource == qubit {
			result = append(result, node)
		}
		for _, ctrl := range node.Controls {
			if ctrl == qubit {
				result = append(result, node)
				break
			}
		}
	}
	return result
}

// MaxStep returns the maximum step index in the DAG.
func (dag *CircuitDAG) MaxStep() int {
	maxStep := 0
	for _, node := range dag.Nodes {
		if node.Step > maxStep {
			maxStep = node.Step
		}
	}
	return maxStep
}

// ToCircuit converts the DAG to a Circuit struct.
func (dag *CircuitDAG) ToCircuit() *Circuit {
	circuit := &Circuit{
		NumQubits: dag.NumQubits,
		Gates:     make([]Gate, 0, len(dag.Nodes)),
		MaxSteps:  dag.MaxStep(),
	}

	// Convert each DAG node to a Gate
	for _, node := range dag.Nodes {
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
		circuit.Gates = append(circuit.Gates, gate)
	}

	return circuit
}

// FromCircuit creates a DAG from a Circuit struct.
func FromCircuit(circuit *Circuit) *CircuitDAG {
	dag := NewCircuitDAG()
	dag.NumQubits = circuit.NumQubits
	dag.NumCbits = circuit.NumCbits()

	// Track the last gate on each qubit to establish dependencies
	lastGateOnQubit := make(map[int]string)

	// Sort gates by step to maintain order
	sortedGates := make([]Gate, len(circuit.Gates))
	copy(sortedGates, circuit.Gates)
	slices.SortFunc(sortedGates, func(a, b Gate) int {
		return a.Step - b.Step
	})

	for _, gate := range sortedGates {
		node := &DAGNode{
			Type:             gate.Type,
			Target:           gate.Target,
			Control:          gate.Control,
			Controls:         gate.Controls,
			MeasureSource:    gate.MeasureSource,
			Step:             gate.Step,
			Params:           gate.Params,
			IsDagger:         gate.IsDagger,
			IsReset:          gate.IsReset,
			ClassicalControl: gate.ClassicalControl,
			IsNoise:          gate.IsNoise,
			NoiseType:        gate.NoiseType,
			Dependencies:     []string{},
		}

		// Establish dependencies based on qubit usage
		qubitsUsed := []int{gate.Target}
		if gate.Control >= 0 {
			qubitsUsed = append(qubitsUsed, gate.Control)
		}
		for _, ctrl := range gate.Controls {
			qubitsUsed = append(qubitsUsed, ctrl)
		}
		if gate.MeasureSource >= 0 {
			qubitsUsed = append(qubitsUsed, gate.MeasureSource)
		}

		// Add dependencies on previous gates using the same qubits
		depSet := make(map[string]bool)
		for _, qubit := range qubitsUsed {
			if lastID, ok := lastGateOnQubit[qubit]; ok {
				depSet[lastID] = true
			}
		}

		for depID := range depSet {
			node.Dependencies = append(node.Dependencies, depID)
		}

		// Generate ID and add to DAG
		node.ID = generateNodeID(gate.Type, gate.Target, gate.Step)
		dag.AddNode(node)

		// Update last gate for each qubit used
		for _, qubit := range qubitsUsed {
			lastGateOnQubit[qubit] = node.ID
		}
	}

	return dag
}

// ToQASM generates QASM 2.0 output from the DAG.
func (dag *CircuitDAG) ToQASM() string {
	// Sort nodes by step for QASM generation
	nodes := dag.TopologicalSort()
	slices.SortFunc(nodes, func(a, b *DAGNode) int {
		return a.Step - b.Step
	})

	// Determine actual qubit count
	maxQubit := dag.NumQubits - 1
	for _, node := range nodes {
		maxQubit = max(maxQubit, node.Target, node.Control, node.MeasureSource)
		for _, ctrl := range node.Controls {
			maxQubit = max(maxQubit, ctrl)
		}
	}
	numQubits := max(maxQubit+1, dag.NumQubits, 1)

	// Determine classical bit count
	numCbits := max(dag.NumCbits, 1)
	for _, node := range nodes {
		if node.Type == "MEASURE" {
			numCbits = max(numCbits, node.Target+1)
		}
		if node.MeasureSource >= 0 {
			numCbits = max(numCbits, node.MeasureSource+1)
		}
		if node.ClassicalControl >= 0 {
			numCbits = max(numCbits, node.ClassicalControl+1)
		}
	}

	var sb strings.Builder
	sb.WriteString("OPENQASM 2.0;\n")
	sb.WriteString("include \"qelib1.inc\";\n\n")
	fmt.Fprintf(&sb, "qreg q[%d];\n", numQubits)
	fmt.Fprintf(&sb, "creg c[%d];\n\n", numCbits)

	// Group nodes by step for output
	stepMap := make(map[int][]*DAGNode)
	maxStep := 0
	for _, node := range nodes {
		stepMap[node.Step] = append(stepMap[node.Step], node)
		if node.Step > maxStep {
			maxStep = node.Step
		}
	}

	// Generate QASM for each step
	for step := 0; step <= maxStep; step++ {
		if stepNodes, ok := stepMap[step]; ok {
			for _, node := range stepNodes {
				dag.writeNodeQASM(&sb, node, numQubits)
			}
		}
	}

	return sb.String()
}

// writeNodeQASM writes a single node's QASM representation.
func (dag *CircuitDAG) writeNodeQASM(sb *strings.Builder, node *DAGNode, numQubits int) {
	switch {
	case node.Type == "BARRIER":
		qubits := make([]string, numQubits)
		for q := 0; q < numQubits; q++ {
			qubits[q] = fmt.Sprintf("q[%d]", q)
		}
		fmt.Fprintf(sb, "barrier %s;\n", strings.Join(qubits, ", "))

	case node.IsNoise:
		if len(node.Params) > 0 {
			fmt.Fprintf(sb, "// noise %s q[%d] param=%s\n", node.NoiseType, node.Target, formatParam(node.Params[0]))
		} else {
			fmt.Fprintf(sb, "// noise %s q[%d]\n", node.NoiseType, node.Target)
		}

	case node.IsReset:
		fmt.Fprintf(sb, "reset q[%d];\n", node.Target)

	case node.ClassicalControl >= 0:
		// Classically controlled gate
		if node.Control >= 0 {
			fmt.Fprintf(sb, "if (c[%d]==1) cx q[%d], q[%d];\n", node.ClassicalControl, node.Control, node.Target)
		} else if len(node.Controls) > 0 {
			gateType := strings.ToLower(node.Type)
			fmt.Fprintf(sb, "if (c[%d]==1) %s ", node.ClassicalControl, gateType)
			for i, ctrl := range node.Controls {
				if i > 0 {
					sb.WriteString(", ")
				}
				fmt.Fprintf(sb, "q[%d]", ctrl)
			}
			fmt.Fprintf(sb, ", q[%d];\n", node.Target)
		} else {
			gateType := strings.ToLower(node.Type)
			if len(node.Params) > 0 {
				fmt.Fprintf(sb, "if (c[%d]==1) %s(%s) q[%d];\n", node.ClassicalControl, gateType, formatParam(node.Params[0]), node.Target)
			} else if node.IsDagger {
				fmt.Fprintf(sb, "if (c[%d]==1) %sdg q[%d];\n", node.ClassicalControl, gateType, node.Target)
			} else {
				fmt.Fprintf(sb, "if (c[%d]==1) %s q[%d];\n", node.ClassicalControl, gateType, node.Target)
			}
		}

	case node.MeasureSource >= 0:
		fmt.Fprintf(sb, "measure q[%d] -> c[%d];\n", node.MeasureSource, node.MeasureSource)
		fmt.Fprintf(sb, "if (c[%d]==1) x q[%d];\n", node.MeasureSource, node.Target)

	case node.Type == "MEASURE":
		fmt.Fprintf(sb, "measure q[%d] -> c[%d];\n", node.Target, node.Target)

	case len(node.Controls) > 0:
		// Multi-controlled gates
		switch node.Type {
		case "CCX", "TOFFOLI":
			if len(node.Controls) >= 2 {
				fmt.Fprintf(sb, "ccx q[%d], q[%d], q[%d];\n", node.Controls[0], node.Controls[1], node.Target)
			}
		default:
			gateType := strings.ToLower(node.Type)
			fmt.Fprintf(sb, "%s ", gateType)
			for i, ctrl := range node.Controls {
				if i > 0 {
					sb.WriteString(", ")
				}
				fmt.Fprintf(sb, "q[%d]", ctrl)
			}
			fmt.Fprintf(sb, ", q[%d];\n", node.Target)
		}

	case node.Control >= 0:
		// Two-qubit gates
		switch node.Type {
		case "CX":
			fmt.Fprintf(sb, "cx q[%d], q[%d];\n", node.Control, node.Target)
		case "CZ":
			fmt.Fprintf(sb, "cz q[%d], q[%d];\n", node.Control, node.Target)
		case "SWAP":
			fmt.Fprintf(sb, "swap q[%d], q[%d];\n", node.Control, node.Target)
		case "CH":
			fmt.Fprintf(sb, "ch q[%d], q[%d];\n", node.Control, node.Target)
		case "CRX":
			if len(node.Params) > 0 {
				fmt.Fprintf(sb, "crx(%s) q[%d], q[%d];\n", formatParam(node.Params[0]), node.Control, node.Target)
			}
		case "CRY":
			if len(node.Params) > 0 {
				fmt.Fprintf(sb, "cry(%s) q[%d], q[%d];\n", formatParam(node.Params[0]), node.Control, node.Target)
			}
		case "CRZ":
			if len(node.Params) > 0 {
				fmt.Fprintf(sb, "crz(%s) q[%d], q[%d];\n", formatParam(node.Params[0]), node.Control, node.Target)
			}
		case "CP", "CU1":
			if len(node.Params) > 0 {
				fmt.Fprintf(sb, "cu1(%s) q[%d], q[%d];\n", formatParam(node.Params[0]), node.Control, node.Target)
			}
		default:
			fmt.Fprintf(sb, "cx q[%d], q[%d];\n", node.Control, node.Target)
		}

	default:
		// Single-qubit gates
		gateType := strings.ToLower(node.Type)
		switch gateType {
		case "rx", "ry", "rz", "p", "u1", "u2", "u3":
			if len(node.Params) == 1 {
				fmt.Fprintf(sb, "%s(%s) q[%d];\n", gateType, formatParam(node.Params[0]), node.Target)
			} else if len(node.Params) == 2 && gateType == "u2" {
				fmt.Fprintf(sb, "%s(%s, %s) q[%d];\n", gateType, formatParam(node.Params[0]), formatParam(node.Params[1]), node.Target)
			} else if len(node.Params) == 3 && gateType == "u3" {
				fmt.Fprintf(sb, "%s(%s, %s, %s) q[%d];\n", gateType, formatParam(node.Params[0]), formatParam(node.Params[1]), formatParam(node.Params[2]), node.Target)
			}
		case "s", "t":
			if node.IsDagger {
				fmt.Fprintf(sb, "%sdg q[%d];\n", gateType, node.Target)
			} else {
				fmt.Fprintf(sb, "%s q[%d];\n", gateType, node.Target)
			}
		case "sx", "sy", "sz":
			if node.IsDagger {
				fmt.Fprintf(sb, "%sdg q[%d];\n", gateType, node.Target)
			} else {
				fmt.Fprintf(sb, "%s q[%d];\n", gateType, node.Target)
			}
		default:
			fmt.Fprintf(sb, "%s q[%d];\n", gateType, node.Target)
		}
	}
}

// ParseQASM parses QASM text and builds a DAG from it.
func (dag *CircuitDAG) ParseQASM(qasm string) error {
	dag.Nodes = make(map[string]*DAGNode)
	dag.rootNodes = []string{}

	lines := strings.Split(qasm, "\n")

	// Classical register map for resolving classical bit references
	cregMap := make(map[string]int)
	cregOffset := 0

	resolveCBit := func(regName, bitIdx string) int {
		startBit, ok := cregMap[regName]
		if !ok {
			if strings.HasPrefix(regName, "c") {
				if idx, err := strconv.Atoi(regName[1:]); err == nil {
					return idx
				}
			}
			return 0
		}
		if bitIdx != "" {
			offset, _ := strconv.Atoi(bitIdx)
			return startBit + offset
		}
		return startBit
	}

	// Track last gate on each qubit for dependency management
	lastGateOnQubit := make(map[int]string)

	// Track qubits used at current step for parallel gate detection
	currentStepQubits := make(map[int]bool)
	currentStep := 0

	// Helper to get qubits used by a node
	getQubitsUsed := func(node *DAGNode) []int {
		qubits := []int{node.Target}
		if node.Control >= 0 {
			qubits = append(qubits, node.Control)
		}
		for _, ctrl := range node.Controls {
			qubits = append(qubits, ctrl)
		}
		if node.MeasureSource >= 0 {
			qubits = append(qubits, node.MeasureSource)
		}
		return qubits
	}

	for i := 0; i < len(lines); i++ {
		line := strings.TrimSpace(lines[i])
		if line == "" {
			continue
		}

		// Skip comments and headers
		if strings.HasPrefix(line, "//") {
			if matches := noiseRegex.FindStringSubmatch(line); matches != nil {
				target, _ := strconv.Atoi(matches[2])
				qubitsUsed := []int{target}
				for _, q := range qubitsUsed {
					if currentStepQubits[q] {
						currentStep++
						currentStepQubits = make(map[int]bool)
						break
					}
				}
				for _, q := range qubitsUsed {
					currentStepQubits[q] = true
				}
				dag.parseNoiseLine(matches, currentStep, lastGateOnQubit)
				currentStep++
				currentStepQubits = make(map[int]bool)
			}
			continue
		}

		if strings.HasPrefix(line, "OPENQASM") || strings.HasPrefix(line, "include") {
			continue
		}

		if strings.HasPrefix(line, "qreg") {
			if matches := qregRegex.FindStringSubmatch(line); len(matches) > 2 {
				n, _ := strconv.Atoi(matches[2])
				dag.NumQubits = n
			}
			continue
		}

		if strings.HasPrefix(line, "creg") {
			if matches := cregRegex.FindStringSubmatch(line); len(matches) > 2 {
				regName := matches[1]
				regSize, _ := strconv.Atoi(matches[2])
				cregMap[regName] = cregOffset
				cregOffset += regSize
			}
			continue
		}

		// Parse various gate types
		node := dag.parseGateLine(line, lines, &i, resolveCBit)
		if node != nil {
			qubitsUsed := getQubitsUsed(node)

			// Barriers always start a new step
			if node.Type == "BARRIER" {
				if len(currentStepQubits) > 0 {
					currentStep++
					currentStepQubits = make(map[int]bool)
				}
				node.Step = currentStep
				currentStep++
				currentStepQubits = make(map[int]bool)
			} else {
				// Check if any qubit conflicts with current step
				conflict := false
				for _, q := range qubitsUsed {
					if currentStepQubits[q] {
						conflict = true
						break
					}
				}

				// Multi-qubit gates always start a new step for clarity
				if node.Control >= 0 || len(node.Controls) > 0 || node.MeasureSource >= 0 {
					if len(currentStepQubits) > 0 {
						currentStep++
						currentStepQubits = make(map[int]bool)
					}
				} else if conflict {
					currentStep++
					currentStepQubits = make(map[int]bool)
				}

				node.Step = currentStep
				for _, q := range qubitsUsed {
					currentStepQubits[q] = true
				}
			}

			// Establish dependencies
			depSet := make(map[string]bool)
			for _, qubit := range qubitsUsed {
				if lastID, ok := lastGateOnQubit[qubit]; ok {
					depSet[lastID] = true
				}
			}
			for depID := range depSet {
				node.Dependencies = append(node.Dependencies, depID)
			}

			node.ID = generateNodeID(node.Type, node.Target, node.Step)
			dag.AddNode(node)

			for _, qubit := range qubitsUsed {
				lastGateOnQubit[qubit] = node.ID
			}
		}
	}

	return nil
}

// parseNoiseLine parses a noise comment line.
func (dag *CircuitDAG) parseNoiseLine(matches []string, step int, lastGateOnQubit map[int]string) *DAGNode {
	noiseType := matches[1]
	target, _ := strconv.Atoi(matches[2])

	var params []float64
	if matches[3] != "" {
		if param, ok := parseParamExpr(matches[3]); ok {
			params = append(params, param)
		}
	}

	node := &DAGNode{
		Type:          "NOISE",
		Target:        target,
		Control:       -1,
		Step:          step,
		Params:        params,
		IsNoise:       true,
		NoiseType:     noiseType,
		MeasureSource: -1,
		Dependencies:  []string{},
	}

	if lastID, ok := lastGateOnQubit[target]; ok {
		node.Dependencies = append(node.Dependencies, lastID)
	}

	node.ID = generateNodeID(node.Type, target, step)
	dag.AddNode(node)
	lastGateOnQubit[target] = node.ID

	return node
}

// parseGateLine parses a single QASM gate line and returns a DAGNode.
func (dag *CircuitDAG) parseGateLine(line string, lines []string, idx *int, resolveCBit func(string, string) int) *DAGNode {
	// Reset gate
	if matches := resetRegex.FindStringSubmatch(line); matches != nil {
		target, _ := strconv.Atoi(matches[1])
		return &DAGNode{
			Type:             "RESET",
			Target:           target,
			Control:          -1,
			MeasureSource:    -1,
			IsReset:          true,
			ClassicalControl: -1,
			Dependencies:     []string{},
		}
	}

	// Barrier
	if barrierRegex.MatchString(line) {
		return &DAGNode{
			Type:             "BARRIER",
			Target:           -1,
			Control:          -1,
			MeasureSource:    -1,
			ClassicalControl: -1,
			Dependencies:     []string{},
		}
	}

	// Measurement with MCX pattern detection
	if matches := measureRegex.FindStringSubmatch(line); matches != nil {
		source, _ := strconv.Atoi(matches[1])
		cbit := resolveCBit(matches[2], matches[3])

		// Check for MCX pattern
		if *idx+1 < len(lines) {
			nextLine := strings.TrimSpace(lines[*idx+1])
			if ifMatches := ifRegex.FindStringSubmatch(nextLine); ifMatches != nil {
				condBit := resolveCBit(ifMatches[1], ifMatches[2])
				target, _ := strconv.Atoi(ifMatches[5])
				if condBit == cbit {
					*idx++
					return &DAGNode{
						Type:             "MCX",
						Target:           target,
						Control:          -1,
						MeasureSource:    source,
						ClassicalControl: -1,
						Dependencies:     []string{},
					}
				}
			}
		}

		return &DAGNode{
			Type:             "MEASURE",
			Target:           source,
			Control:          -1,
			MeasureSource:    -1,
			ClassicalControl: -1,
			Dependencies:     []string{},
		}
	}

	// Classically controlled parameterized gates
	if matches := ifParamRegex.FindStringSubmatch(line); matches != nil {
		cbit := resolveCBit(matches[1], matches[2])
		gateType := strings.ToUpper(matches[4])
		param, _ := parseParamExpr(matches[5])
		target, _ := strconv.Atoi(matches[6])
		return &DAGNode{
			Type:             gateType,
			Target:           target,
			Control:          -1,
			Params:           []float64{param},
			ClassicalControl: cbit,
			MeasureSource:    -1,
			Dependencies:     []string{},
		}
	}

	// Classically controlled gates
	if matches := ifRegex.FindStringSubmatch(line); matches != nil {
		cbit := resolveCBit(matches[1], matches[2])
		gateType := strings.ToUpper(matches[4])
		target, _ := strconv.Atoi(matches[5])
		return &DAGNode{
			Type:             gateType,
			Target:           target,
			Control:          -1,
			ClassicalControl: cbit,
			MeasureSource:    -1,
			Dependencies:     []string{},
		}
	}

	// Three-qubit gates
	if matches := threeQubitRegex.FindStringSubmatch(line); matches != nil {
		gateType := strings.ToUpper(matches[1])
		qubit1, _ := strconv.Atoi(matches[2])
		qubit2, _ := strconv.Atoi(matches[3])
		qubit3, _ := strconv.Atoi(matches[4])
		return &DAGNode{
			Type:             gateType,
			Target:           qubit3,
			Control:          -1,
			Controls:         []int{qubit1, qubit2},
			ClassicalControl: -1,
			MeasureSource:    -1,
			Dependencies:     []string{},
		}
	}

	// Two-qubit parameterized gates
	if matches := twoQubitParamRegex.FindStringSubmatch(line); matches != nil {
		gateType := strings.ToUpper(matches[1])
		param, _ := parseParamExpr(matches[2])
		qubit1, _ := strconv.Atoi(matches[3])
		qubit2, _ := strconv.Atoi(matches[4])
		return &DAGNode{
			Type:             gateType,
			Target:           qubit2,
			Control:          qubit1,
			Params:           []float64{param},
			ClassicalControl: -1,
			MeasureSource:    -1,
			Dependencies:     []string{},
		}
	}

	// Two-qubit gates
	if matches := twoQubitRegex.FindStringSubmatch(line); matches != nil {
		gateType := strings.ToUpper(matches[1])
		qubit1, _ := strconv.Atoi(matches[2])
		qubit2, _ := strconv.Atoi(matches[3])
		return &DAGNode{
			Type:             gateType,
			Target:           qubit2,
			Control:          qubit1,
			ClassicalControl: -1,
			MeasureSource:    -1,
			Dependencies:     []string{},
		}
	}

	// Single-qubit parameterized gates
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

		return &DAGNode{
			Type:             gateType,
			Target:           target,
			Control:          -1,
			Params:           params,
			ClassicalControl: -1,
			MeasureSource:    -1,
			Dependencies:     []string{},
		}
	}

	// Single-qubit gates (including dagger gates)
	if matches := singleGateRegex.FindStringSubmatch(line); matches != nil {
		gateType := strings.ToUpper(matches[1])
		target, _ := strconv.Atoi(matches[2])

		// Check for dagger gates
		isDagger := false
		if strings.HasSuffix(gateType, "DG") || strings.HasSuffix(gateType, "dg") {
			isDagger = true
			gateType = strings.TrimSuffix(gateType, "DG")
			gateType = strings.TrimSuffix(gateType, "dg")
		}

		// Handle square root gates with dagger
		baseGate := gateType
		if strings.HasPrefix(gateType, "SX") || strings.HasPrefix(gateType, "SY") || strings.HasPrefix(gateType, "SZ") {
			baseGate = gateType
			if strings.HasSuffix(gateType, "DG") || strings.HasSuffix(gateType, "dg") {
				isDagger = true
				baseGate = strings.TrimSuffix(gateType, "DG")
				baseGate = strings.TrimSuffix(baseGate, "dg")
			}
		}

		return &DAGNode{
			Type:             baseGate,
			Target:           target,
			Control:          -1,
			IsDagger:         isDagger,
			ClassicalControl: -1,
			MeasureSource:    -1,
			Dependencies:     []string{},
		}
	}

	return nil
}

// Clone creates a deep copy of the DAG.
func (dag *CircuitDAG) Clone() *CircuitDAG {
	clone := NewCircuitDAG()
	clone.NumQubits = dag.NumQubits
	clone.NumCbits = dag.NumCbits

	for id, node := range dag.Nodes {
		newNode := &DAGNode{
			ID:               node.ID,
			Type:             node.Type,
			Target:           node.Target,
			Control:          node.Control,
			Controls:         append([]int{}, node.Controls...),
			MeasureSource:    node.MeasureSource,
			Step:             node.Step,
			Params:           append([]float64{}, node.Params...),
			IsDagger:         node.IsDagger,
			IsReset:          node.IsReset,
			ClassicalControl: node.ClassicalControl,
			IsNoise:          node.IsNoise,
			NoiseType:        node.NoiseType,
			Dependencies:     append([]string{}, node.Dependencies...),
		}
		clone.Nodes[id] = newNode
	}

	clone.updateRootNodes()
	return clone
}

// GetNodeAt returns the node at the given step and qubit, if any.
func (dag *CircuitDAG) GetNodeAt(step, qubit int) *DAGNode {
	for _, node := range dag.Nodes {
		if node.Step == step {
			if node.Target == qubit || node.Control == qubit || node.MeasureSource == qubit {
				return node
			}
			for _, ctrl := range node.Controls {
				if ctrl == qubit {
					return node
				}
			}
		}
	}
	return nil
}

// CanPlaceGateAt checks if a gate can be placed at the given step using the specified qubits.
// Returns false if any qubit is already used by a multi-qubit gate or barrier at that step.
func (dag *CircuitDAG) CanPlaceGateAt(step int, qubits []int) bool {
	for _, qubit := range qubits {
		node := dag.GetNodeAt(step, qubit)
		if node == nil {
			continue
		}
		if node.Type == "BARRIER" {
			return false
		}
		if node.Control >= 0 || len(node.Controls) > 0 || node.MeasureSource >= 0 {
			return false
		}
	}
	return true
}

// RemoveNodeAt removes a node at the given step and qubit.
func (dag *CircuitDAG) RemoveNodeAt(step, qubit int) {
	node := dag.GetNodeAt(step, qubit)
	if node != nil {
		dag.RemoveNode(node.ID)
	}
}

// RemoveNodesOnQubit removes all nodes that reference a specific qubit.
func (dag *CircuitDAG) RemoveNodesOnQubit(qubit int) {
	toRemove := []string{}
	for id, node := range dag.Nodes {
		if node.Target == qubit || node.Control == qubit || node.MeasureSource == qubit {
			toRemove = append(toRemove, id)
			continue
		}
		for _, ctrl := range node.Controls {
			if ctrl == qubit {
				toRemove = append(toRemove, id)
				break
			}
		}
	}
	for _, id := range toRemove {
		dag.RemoveNode(id)
	}
}

// AddGate adds a gate to the DAG at the specified step.
func (dag *CircuitDAG) AddGate(gateType string, target, step int, control ...int) {
	ctrl := -1
	if len(control) > 0 {
		ctrl = control[0]
	}

	node := &DAGNode{
		Type:             gateType,
		Target:           target,
		Control:          ctrl,
		Step:             step,
		ClassicalControl: -1,
		MeasureSource:    -1,
		Dependencies:     []string{},
	}

	// Establish dependencies
	lastGateOnQubit := make(map[int]string)
	for _, n := range dag.Nodes {
		qubits := []int{n.Target}
		if n.Control >= 0 {
			qubits = append(qubits, n.Control)
		}
		for _, c := range n.Controls {
			qubits = append(qubits, c)
		}
		if n.MeasureSource >= 0 {
			qubits = append(qubits, n.MeasureSource)
		}
		for _, q := range qubits {
			if n.Step < step || (n.Step == step && n.Type < gateType) {
				lastGateOnQubit[q] = n.ID
			}
		}
	}

	qubitsUsed := []int{target}
	if ctrl >= 0 {
		qubitsUsed = append(qubitsUsed, ctrl)
	}

	depSet := make(map[string]bool)
	for _, qubit := range qubitsUsed {
		if lastID, ok := lastGateOnQubit[qubit]; ok {
			depSet[lastID] = true
		}
	}
	for depID := range depSet {
		node.Dependencies = append(node.Dependencies, depID)
	}

	node.ID = generateNodeID(gateType, target, step)
	dag.AddNode(node)
}

// AddParameterizedGate adds a parameterized gate to the DAG.
func (dag *CircuitDAG) AddParameterizedGate(gateType string, target, step int, params []float64, control ...int) {
	ctrl := -1
	if len(control) > 0 {
		ctrl = control[0]
	}

	node := &DAGNode{
		Type:             gateType,
		Target:           target,
		Control:          ctrl,
		Step:             step,
		Params:           params,
		ClassicalControl: -1,
		MeasureSource:    -1,
		Dependencies:     []string{},
	}

	// Establish dependencies (same as AddGate)
	lastGateOnQubit := make(map[int]string)
	for _, n := range dag.Nodes {
		qubits := []int{n.Target}
		if n.Control >= 0 {
			qubits = append(qubits, n.Control)
		}
		for _, c := range n.Controls {
			qubits = append(qubits, c)
		}
		if n.MeasureSource >= 0 {
			qubits = append(qubits, n.MeasureSource)
		}
		for _, q := range qubits {
			if n.Step < step || (n.Step == step && n.Type < gateType) {
				lastGateOnQubit[q] = n.ID
			}
		}
	}

	qubitsUsed := []int{target}
	if ctrl >= 0 {
		qubitsUsed = append(qubitsUsed, ctrl)
	}

	depSet := make(map[string]bool)
	for _, qubit := range qubitsUsed {
		if lastID, ok := lastGateOnQubit[qubit]; ok {
			depSet[lastID] = true
		}
	}
	for depID := range depSet {
		node.Dependencies = append(node.Dependencies, depID)
	}

	node.ID = generateNodeID(gateType, target, step)
	dag.AddNode(node)
}

// AddMultiControlGate adds a multi-controlled gate to the DAG.
func (dag *CircuitDAG) AddMultiControlGate(gateType string, target, step int, controls []int) {
	node := &DAGNode{
		Type:             gateType,
		Target:           target,
		Control:          -1,
		Controls:         controls,
		Step:             step,
		ClassicalControl: -1,
		MeasureSource:    -1,
		Dependencies:     []string{},
	}

	// Establish dependencies
	lastGateOnQubit := make(map[int]string)
	for _, n := range dag.Nodes {
		qubits := []int{n.Target}
		if n.Control >= 0 {
			qubits = append(qubits, n.Control)
		}
		for _, c := range n.Controls {
			qubits = append(qubits, c)
		}
		if n.MeasureSource >= 0 {
			qubits = append(qubits, n.MeasureSource)
		}
		for _, q := range qubits {
			if n.Step < step || (n.Step == step && n.Type < gateType) {
				lastGateOnQubit[q] = n.ID
			}
		}
	}

	qubitsUsed := []int{target}
	qubitsUsed = append(qubitsUsed, controls...)

	depSet := make(map[string]bool)
	for _, qubit := range qubitsUsed {
		if lastID, ok := lastGateOnQubit[qubit]; ok {
			depSet[lastID] = true
		}
	}
	for depID := range depSet {
		node.Dependencies = append(node.Dependencies, depID)
	}

	node.ID = generateNodeID(gateType, target, step)
	dag.AddNode(node)
}

// AddClassicalControlGate adds a classically-controlled gate to the DAG.
func (dag *CircuitDAG) AddClassicalControlGate(gateType string, target, step, cbit int) {
	node := &DAGNode{
		Type:             gateType,
		Target:           target,
		Control:          -1,
		Step:             step,
		ClassicalControl: cbit,
		MeasureSource:    -1,
		Dependencies:     []string{},
	}

	// Update classical bit count
	if cbit+1 > dag.NumCbits {
		dag.NumCbits = cbit + 1
	}

	// Establish dependencies
	lastGateOnQubit := make(map[int]string)
	for _, n := range dag.Nodes {
		qubits := []int{n.Target}
		if n.Control >= 0 {
			qubits = append(qubits, n.Control)
		}
		for _, c := range n.Controls {
			qubits = append(qubits, c)
		}
		if n.MeasureSource >= 0 {
			qubits = append(qubits, n.MeasureSource)
		}
		for _, q := range qubits {
			if n.Step < step || (n.Step == step && n.Type < gateType) {
				lastGateOnQubit[q] = n.ID
			}
		}
	}

	if lastID, ok := lastGateOnQubit[target]; ok {
		node.Dependencies = append(node.Dependencies, lastID)
	}

	node.ID = generateNodeID(gateType, target, step)
	dag.AddNode(node)
}

// AddDaggerGate adds a dagger gate to the DAG.
func (dag *CircuitDAG) AddDaggerGate(gateType string, target, step int) {
	node := &DAGNode{
		Type:             gateType,
		Target:           target,
		Control:          -1,
		Step:             step,
		IsDagger:         true,
		ClassicalControl: -1,
		MeasureSource:    -1,
		Dependencies:     []string{},
	}

	// Establish dependencies
	lastGateOnQubit := make(map[int]string)
	for _, n := range dag.Nodes {
		qubits := []int{n.Target}
		if n.Control >= 0 {
			qubits = append(qubits, n.Control)
		}
		for _, c := range n.Controls {
			qubits = append(qubits, c)
		}
		if n.MeasureSource >= 0 {
			qubits = append(qubits, n.MeasureSource)
		}
		for _, q := range qubits {
			if n.Step < step || (n.Step == step && n.Type < gateType) {
				lastGateOnQubit[q] = n.ID
			}
		}
	}

	if lastID, ok := lastGateOnQubit[target]; ok {
		node.Dependencies = append(node.Dependencies, lastID)
	}

	node.ID = generateNodeID(gateType, target, step)
	dag.AddNode(node)
}

// AddReset adds a reset operation to the DAG.
func (dag *CircuitDAG) AddReset(target, step int) {
	node := &DAGNode{
		Type:             "RESET",
		Target:           target,
		Control:          -1,
		Step:             step,
		IsReset:          true,
		ClassicalControl: -1,
		MeasureSource:    -1,
		Dependencies:     []string{},
	}

	// Establish dependencies
	lastGateOnQubit := make(map[int]string)
	for _, n := range dag.Nodes {
		qubits := []int{n.Target}
		if n.Control >= 0 {
			qubits = append(qubits, n.Control)
		}
		for _, c := range n.Controls {
			qubits = append(qubits, c)
		}
		if n.MeasureSource >= 0 {
			qubits = append(qubits, n.MeasureSource)
		}
		for _, q := range qubits {
			if n.Step < step || (n.Step == step && n.Type < "RESET") {
				lastGateOnQubit[q] = n.ID
			}
		}
	}

	if lastID, ok := lastGateOnQubit[target]; ok {
		node.Dependencies = append(node.Dependencies, lastID)
	}

	node.ID = generateNodeID("RESET", target, step)
	dag.AddNode(node)
}

// AddNoise adds a noise operation to the DAG.
func (dag *CircuitDAG) AddNoise(target, step int, noiseType string, params ...float64) {
	node := &DAGNode{
		Type:             "NOISE",
		Target:           target,
		Control:          -1,
		Step:             step,
		Params:           params,
		IsNoise:          true,
		NoiseType:        noiseType,
		ClassicalControl: -1,
		MeasureSource:    -1,
		Dependencies:     []string{},
	}

	// Establish dependencies
	lastGateOnQubit := make(map[int]string)
	for _, n := range dag.Nodes {
		qubits := []int{n.Target}
		if n.Control >= 0 {
			qubits = append(qubits, n.Control)
		}
		for _, c := range n.Controls {
			qubits = append(qubits, c)
		}
		if n.MeasureSource >= 0 {
			qubits = append(qubits, n.MeasureSource)
		}
		for _, q := range qubits {
			if n.Step < step || (n.Step == step && n.Type < "NOISE") {
				lastGateOnQubit[q] = n.ID
			}
		}
	}

	if lastID, ok := lastGateOnQubit[target]; ok {
		node.Dependencies = append(node.Dependencies, lastID)
	}

	node.ID = generateNodeID("NOISE", target, step)
	dag.AddNode(node)
}

// AddMeasureControlGate adds a measurement-controlled gate to the DAG.
func (dag *CircuitDAG) AddMeasureControlGate(source, target, step int) {
	node := &DAGNode{
		Type:             "MCX",
		Target:           target,
		Control:          -1,
		MeasureSource:    source,
		Step:             step,
		ClassicalControl: -1,
		Dependencies:     []string{},
	}

	// Update classical bit count
	if source+1 > dag.NumCbits {
		dag.NumCbits = source + 1
	}

	// Establish dependencies on both source and target qubits
	lastGateOnQubit := make(map[int]string)
	for _, n := range dag.Nodes {
		qubits := []int{n.Target}
		if n.Control >= 0 {
			qubits = append(qubits, n.Control)
		}
		for _, c := range n.Controls {
			qubits = append(qubits, c)
		}
		if n.MeasureSource >= 0 {
			qubits = append(qubits, n.MeasureSource)
		}
		for _, q := range qubits {
			if n.Step < step || (n.Step == step && n.Type < "MCX") {
				lastGateOnQubit[q] = n.ID
			}
		}
	}

	depSet := make(map[string]bool)
	if lastID, ok := lastGateOnQubit[source]; ok {
		depSet[lastID] = true
	}
	if lastID, ok := lastGateOnQubit[target]; ok {
		depSet[lastID] = true
	}
	for depID := range depSet {
		node.Dependencies = append(node.Dependencies, depID)
	}

	node.ID = generateNodeID("MCX", target, step)
	dag.AddNode(node)
}

// AddBarrier adds a barrier spanning all qubits at the given step.
func (dag *CircuitDAG) AddBarrier(step int) {
	// Remove any existing barrier at this step
	toRemove := []string{}
	for id, node := range dag.Nodes {
		if node.Step == step && node.Type == "BARRIER" {
			toRemove = append(toRemove, id)
		}
	}
	for _, id := range toRemove {
		dag.RemoveNode(id)
	}

	node := &DAGNode{
		Type:             "BARRIER",
		Target:           -1,
		Control:          -1,
		Step:             step,
		ClassicalControl: -1,
		MeasureSource:    -1,
		Dependencies:     []string{},
	}

	node.ID = generateNodeID("BARRIER", -1, step)
	dag.AddNode(node)
}

// NumCbits returns the number of classical bits.
func (dag *CircuitDAG) NumCbitsInt() int {
	return dag.NumCbits
}
