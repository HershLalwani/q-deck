# Q-Deck v0.2.0

TUI application built using [bubbletea](https://github.com/charmbracelet/bubbletea) to build quantum circuits and generate QASM code.

## Features (v0.2.0)

### Gate Library
- **Single Qubit**: H, X, Y, Z, I, S, S†, T, T†, √X (SX), √Y (SY)
- **Rotation Gates**: RX, RY, RZ, P (phase), U1, U2, U3
- **Multi-Qubit**: CNOT, CZ, CH, SWAP, Toffoli (CCX)
- **Controlled Rotations**: CRX, CRY, CRZ, CU1
- **Measurement**: Measure, MCX (measurement-controlled X)
- **Special**: Reset, Barrier
- **Noise Models**: Depolarizing, Amplitude Damping, Phase Damping

### Features
- Interactive circuit editing with keyboard navigation
- Parameterized gates with pi notation support (e.g., pi/2, 3*pi/4)
- Real-time QASM generation and editing
- Gate editing (modify parameters, target, controls)
- Classical control support

## Usage

```bash
# Build the application
go build -o q-deck

# Run the application
./q-deck

# Run tests
go test -v
```

## Controls

- **Arrow keys / hjkl**: Navigate circuit
- **a**: Add gate menu
- **e**: Edit gate at cursor
- **Delete/Backspace**: Remove gate
- **+/-**: Add/remove qubits
- **Ctrl+S**: Save QASM to file
- **q / Ctrl+C**: Quit

## QASM Support

The application supports OpenQASM 2.0 format with extensions for:
- Parameterized gates with pi notation
- Classical-controlled operations
- Noise operations (as comments)
- All standard quantum gates
