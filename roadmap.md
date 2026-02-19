## Basic Roadmap for Q-Deck

### v0.1.0
- [x] Basic gates (H, X, Y, Z)
- [x] Measurement
- [x] CNOT, CZ, SWAP gates

### v0.2.0 
- [x] Extended Set of Operators
  - [x] Phase gates (S, T, S†, T†)
  - [x] Toffoli (CCX) 3-qubit gate
  - [x] Rotation gates (RX, RY, RZ)
  - [x] Square root gates (√X, √Y)
  - [x] Controlled rotations (CRX, CRY, CRZ, CH)
  - [x] Universal gates (U1, U2, U3, P)
- [x] Reset gate
- [x] Conditional gates (classical-controlled operations with `if (c[i]==1)`)
- [x] Parameterized gates (gates with variable angles)
- [x] Noise models
  - [x] Depolarizing noise
  - [x] Amplitude damping
  - [x] Phase damping

### v0.3.0
- [ ] Custom gates (user-defined unitary operations)
- [ ] Visualization tools (state vector visualization)
- [ ] Quantum algorithms (sample circuits)
- [ ] Exporting to other formats (OpenQASM, Qiskit)
- [ ] Show state values, probabilities, and measurement outcomes
- [ ] Bloch sphere states