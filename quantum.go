package main

import (
	"math"
	"math/cmplx"
)

type Complex = complex128

type StateVector struct {
	Amplitudes []Complex
	NumQubits  int
}

func NewStateVector(numQubits int) *StateVector {
	n := 1 << numQubits
	amps := make([]Complex, n)
	amps[0] = 1
	return &StateVector{Amplitudes: amps, NumQubits: numQubits}
}

func (s *StateVector) Clone() *StateVector {
	amps := make([]Complex, len(s.Amplitudes))
	copy(amps, s.Amplitudes)
	return &StateVector{Amplitudes: amps, NumQubits: s.NumQubits}
}

func (s *StateVector) ApplyGate(gateType string, target int, control int, params []float64) {
	switch gateType {
	case "H":
		s.applyH(target)
	case "X":
		s.applyX(target)
	case "Y":
		s.applyY(target)
	case "Z":
		s.applyZ(target)
	case "S":
		s.applyS(target, false)
	case "SDG", "Sdg":
		s.applyS(target, true)
	case "T":
		s.applyT(target, false)
	case "TDG", "Tdg":
		s.applyT(target, true)
	case "RX":
		theta := 0.0
		if len(params) > 0 {
			theta = params[0]
		}
		s.applyRX(target, theta)
	case "RY":
		theta := 0.0
		if len(params) > 0 {
			theta = params[0]
		}
		s.applyRY(target, theta)
	case "RZ":
		theta := 0.0
		if len(params) > 0 {
			theta = params[0]
		}
		s.applyRZ(target, theta)
	case "P", "U1":
		theta := 0.0
		if len(params) > 0 {
			theta = params[0]
		}
		s.applyRZ(target, theta)
	case "CX":
		if control >= 0 {
			s.applyCX(control, target)
		}
	case "CZ":
		if control >= 0 {
			s.applyCZ(control, target)
		}
	case "SWAP":
		if control >= 0 {
			s.applySWAP(control, target)
		}
	case "RESET":
		s.applyReset(target)
	case "MEASURE":
	}
}

func (s *StateVector) applyH(q int) {
	hFactor := complex(1.0/math.Sqrt2, 0)
	n := len(s.Amplitudes)
	bit := 1 << q
	newAmps := make([]Complex, n)
	for i := 0; i < n; i++ {
		if i&bit == 0 {
			j := i | bit
			newAmps[i] = hFactor * (s.Amplitudes[i] + s.Amplitudes[j])
			newAmps[j] = hFactor * (s.Amplitudes[i] - s.Amplitudes[j])
		}
	}
	s.Amplitudes = newAmps
}

func (s *StateVector) applyX(q int) {
	n := len(s.Amplitudes)
	bit := 1 << q
	for i := 0; i < n; i++ {
		if i&bit == 0 {
			j := i | bit
			s.Amplitudes[i], s.Amplitudes[j] = s.Amplitudes[j], s.Amplitudes[i]
		}
	}
}

func (s *StateVector) applyY(q int) {
	n := len(s.Amplitudes)
	bit := 1 << q
	for i := 0; i < n; i++ {
		if i&bit == 0 {
			j := i | bit
			s.Amplitudes[i], s.Amplitudes[j] = 1i*s.Amplitudes[j], -1i*s.Amplitudes[i]
		}
	}
}

func (s *StateVector) applyZ(q int) {
	n := len(s.Amplitudes)
	bit := 1 << q
	for i := 0; i < n; i++ {
		if i&bit != 0 {
			s.Amplitudes[i] *= -1
		}
	}
}

func (s *StateVector) applyS(q int, dagger bool) {
	n := len(s.Amplitudes)
	bit := 1 << q
	factor := 1i
	if dagger {
		factor = -1i
	}
	for i := 0; i < n; i++ {
		if i&bit != 0 {
			s.Amplitudes[i] *= factor
		}
	}
}

func (s *StateVector) applyT(q int, dagger bool) {
	n := len(s.Amplitudes)
	bit := 1 << q
	var factor Complex
	if dagger {
		factor = cmplx.Exp(complex(0, -math.Pi/4))
	} else {
		factor = cmplx.Exp(complex(0, math.Pi/4))
	}
	for i := 0; i < n; i++ {
		if i&bit != 0 {
			s.Amplitudes[i] *= factor
		}
	}
}

func (s *StateVector) applyRX(q int, theta float64) {
	n := len(s.Amplitudes)
	bit := 1 << q
	c := complex(math.Cos(theta/2), 0)
	js := complex(0, -math.Sin(theta/2))
	newAmps := make([]Complex, n)
	for i := 0; i < n; i++ {
		if i&bit == 0 {
			j := i | bit
			newAmps[i] = c*s.Amplitudes[i] + js*s.Amplitudes[j]
			newAmps[j] = js*s.Amplitudes[i] + c*s.Amplitudes[j]
		}
	}
	s.Amplitudes = newAmps
}

func (s *StateVector) applyRY(q int, theta float64) {
	n := len(s.Amplitudes)
	bit := 1 << q
	c := complex(math.Cos(theta/2), 0)
	s_ := complex(math.Sin(theta/2), 0)
	newAmps := make([]Complex, n)
	for i := 0; i < n; i++ {
		if i&bit == 0 {
			j := i | bit
			newAmps[i] = c*s.Amplitudes[i] - s_*s.Amplitudes[j]
			newAmps[j] = s_*s.Amplitudes[i] + c*s.Amplitudes[j]
		}
	}
	s.Amplitudes = newAmps
}

func (s *StateVector) applyRZ(q int, theta float64) {
	n := len(s.Amplitudes)
	bit := 1 << q
	phase := cmplx.Exp(complex(0, theta/2))
	for i := 0; i < n; i++ {
		if i&bit != 0 {
			s.Amplitudes[i] *= phase
		} else {
			s.Amplitudes[i] *= cmplx.Conj(phase)
		}
	}
}

func (s *StateVector) applyCX(control, target int) {
	n := len(s.Amplitudes)
	cBit := 1 << control
	tBit := 1 << target
	for i := 0; i < n; i++ {
		if i&cBit != 0 && i&tBit == 0 {
			j := i | tBit
			s.Amplitudes[i], s.Amplitudes[j] = s.Amplitudes[j], s.Amplitudes[i]
		}
	}
}

func (s *StateVector) applyCZ(control, target int) {
	n := len(s.Amplitudes)
	cBit := 1 << control
	tBit := 1 << target
	for i := 0; i < n; i++ {
		if i&cBit != 0 && i&tBit != 0 {
			s.Amplitudes[i] *= -1
		}
	}
}

func (s *StateVector) applySWAP(q1, q2 int) {
	n := len(s.Amplitudes)
	bit1 := 1 << q1
	bit2 := 1 << q2
	for i := 0; i < n; i++ {
		if i&bit1 != 0 && i&bit2 == 0 {
			j := (i & ^bit1) | bit2
			s.Amplitudes[i], s.Amplitudes[j] = s.Amplitudes[j], s.Amplitudes[i]
		}
	}
}

func (s *StateVector) applyReset(q int) {
	n := len(s.Amplitudes)
	bit := 1 << q

	prob0 := 0.0
	for i := 0; i < n; i++ {
		if i&bit == 0 {
			prob0 += real(s.Amplitudes[i] * cmplx.Conj(s.Amplitudes[i]))
		}
	}

	norm := 1.0
	if prob0 > 0 {
		norm = math.Sqrt(prob0)
	}

	for i := 0; i < n; i++ {
		if i&bit == 0 {
			s.Amplitudes[i] = s.Amplitudes[i] / complex(norm, 0)
		} else {
			s.Amplitudes[i] = 0
		}
	}
}

type QubitProbability struct {
	Prob0 float64
	Prob1 float64
}

func (s *StateVector) GetQubitProbabilities() []QubitProbability {
	probs := make([]QubitProbability, s.NumQubits)
	n := len(s.Amplitudes)

	for i := 0; i < n; i++ {
		prob := real(s.Amplitudes[i] * cmplx.Conj(s.Amplitudes[i]))
		for q := 0; q < s.NumQubits; q++ {
			if i&(1<<q) != 0 {
				probs[q].Prob1 += prob
			} else {
				probs[q].Prob0 += prob
			}
		}
	}

	return probs
}

func SimulateCircuit(circuit *Circuit, upToStep int) *StateVector {
	if circuit.NumQubits == 0 {
		return NewStateVector(1)
	}
	state := NewStateVector(circuit.NumQubits)

	gates := make([]Gate, len(circuit.Gates))
	copy(gates, circuit.Gates)

	for i := range gates {
		for j := i + 1; j < len(gates); j++ {
			if gates[j].Step < gates[i].Step {
				gates[i], gates[j] = gates[j], gates[i]
			}
		}
	}

	for _, gate := range gates {
		if upToStep >= 0 && gate.Step > upToStep {
			continue
		}
		if gate.Type == "BARRIER" || gate.Type == "MEASURE" || gate.Type == "MCX" {
			continue
		}
		if gate.IsNoise {
			continue
		}
		if gate.ClassicalControl >= 0 {
			continue
		}

		if len(gate.Controls) > 0 {
			for _, ctrl := range gate.Controls {
				state.ApplyGate(gate.Type, gate.Target, ctrl, gate.Params)
			}
		} else {
			state.ApplyGate(gate.Type, gate.Target, gate.Control, gate.Params)
		}
	}

	return state
}

type QSphereState struct {
	BasisState int
	Amplitude  Complex
	Prob       float64
	Phase      float64
	Hamming    int
}

func (s *StateVector) GetQSphereStates() []QSphereState {
	n := len(s.Amplitudes)
	states := make([]QSphereState, 0, n)

	for i := 0; i < n; i++ {
		amp := s.Amplitudes[i]
		prob := real(amp * cmplx.Conj(amp))

		if prob > 1e-10 {
			phase := cmplx.Phase(amp)
			hamming := bitsCount(i)
			states = append(states, QSphereState{
				BasisState: i,
				Amplitude:  amp,
				Prob:       prob,
				Phase:      phase,
				Hamming:    hamming,
			})
		}
	}

	return states
}

func bitsCount(x int) int {
	count := 0
	for x > 0 {
		count += x & 1
		x >>= 1
	}
	return count
}
