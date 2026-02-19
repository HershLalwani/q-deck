package main

import (
	"fmt"
	"strings"
)

// parameterHint provides a hint for parameter input
type parameterHint struct {
	required bool
	example  string
}

// menuItem represents a single gate choice in the menu.
type menuItem struct {
	name        string
	gateType    string
	symbol      string
	needsTarget bool
	needsParams bool
	paramHint   parameterHint
}

// menuCategory groups related menu items under a tab.
type menuCategory struct {
	name  string
	items []menuItem
}

// gateMenu defines the gate picker categories and items.
var gateMenu = []menuCategory{
	{
		name: "Single Qubit",
		items: []menuItem{
			{name: "Hadamard", gateType: "H", symbol: "H"},
			{name: "Pauli-X (NOT)", gateType: "X", symbol: "X"},
			{name: "Pauli-Y", gateType: "Y", symbol: "Y"},
			{name: "Pauli-Z", gateType: "Z", symbol: "Z"},
			{name: "Identity", gateType: "I", symbol: "I"},
			{name: "Phase (S)", gateType: "S", symbol: "S"},
			{name: "Phase Dagger (S†)", gateType: "SDG", symbol: "S†"},
			{name: "T Gate", gateType: "T", symbol: "T"},
			{name: "T Dagger (T†)", gateType: "TDG", symbol: "T†"},
			{name: "√X (SX)", gateType: "SX", symbol: "√X"},
			{name: "√Y (SY)", gateType: "SY", symbol: "√Y"},
		},
	},
	{
		name: "Rotation",
		items: []menuItem{
			{name: "Rotate X", gateType: "RX", symbol: "RX", needsParams: true, paramHint: parameterHint{required: true, example: "pi/2"}},
			{name: "Rotate Y", gateType: "RY", symbol: "RY", needsParams: true, paramHint: parameterHint{required: true, example: "pi/2"}},
			{name: "Rotate Z", gateType: "RZ", symbol: "RZ", needsParams: true, paramHint: parameterHint{required: true, example: "pi/2"}},
			{name: "Phase Shift", gateType: "P", symbol: "P", needsParams: true, paramHint: parameterHint{required: true, example: "pi/4"}},
			{name: "Universal U1", gateType: "U1", symbol: "U1", needsParams: true, paramHint: parameterHint{required: true, example: "lambda"}},
			{name: "Universal U2", gateType: "U2", symbol: "U2", needsParams: true, paramHint: parameterHint{required: true, example: "phi,lambda"}},
			{name: "Universal U3", gateType: "U3", symbol: "U3", needsParams: true, paramHint: parameterHint{required: true, example: "theta,phi,lambda"}},
		},
	},
	{
		name: "Multi Qubit",
		items: []menuItem{
			{name: "CNOT", gateType: "CX", symbol: "●─⊕", needsTarget: true},
			{name: "Controlled-Z", gateType: "CZ", symbol: "●─●", needsTarget: true},
			{name: "Controlled-H", gateType: "CH", symbol: "●─H", needsTarget: true},
			{name: "SWAP", gateType: "SWAP", symbol: "×─×", needsTarget: true},
			{name: "Toffoli (CCX)", gateType: "CCX", symbol: "●─●─⊕", needsTarget: true},
			{name: "C-Rotate X", gateType: "CRX", symbol: "●─RX", needsTarget: true, needsParams: true, paramHint: parameterHint{required: true, example: "pi/2"}},
			{name: "C-Rotate Y", gateType: "CRY", symbol: "●─RY", needsTarget: true, needsParams: true, paramHint: parameterHint{required: true, example: "pi/2"}},
			{name: "C-Rotate Z", gateType: "CRZ", symbol: "●─RZ", needsTarget: true, needsParams: true, paramHint: parameterHint{required: true, example: "pi/2"}},
			{name: "C-Phase (CU1)", gateType: "CU1", symbol: "●─U1", needsTarget: true, needsParams: true, paramHint: parameterHint{required: true, example: "lambda"}},
		},
	},
	{
		name: "Measurement",
		items: []menuItem{
			{name: "Measure", gateType: "MEASURE", symbol: "M"},
			{name: "Measure-Ctrl X", gateType: "MCX", symbol: "M─⊕", needsTarget: true},
		},
	},
	{
		name: "Special",
		items: []menuItem{
			{name: "Reset", gateType: "RESET", symbol: "|0⟩"},
			{name: "Barrier", gateType: "BARRIER", symbol: "┃"},
		},
	},
	{
		name: "Noise",
		items: []menuItem{
			{name: "Depolarizing", gateType: "NOISE_DEPOL", symbol: "N", needsParams: true, paramHint: parameterHint{required: false, example: "0.01"}},
			{name: "Amplitude Damping", gateType: "NOISE_AMP", symbol: "N", needsParams: true, paramHint: parameterHint{required: false, example: "0.01"}},
			{name: "Phase Damping", gateType: "NOISE_PHASE", symbol: "N", needsParams: true, paramHint: parameterHint{required: false, example: "0.01"}},
		},
	},
}

// renderMenu renders the floating gate-picker popup.
func (m Model) renderMenu() string {
	var sb strings.Builder

	sb.WriteString(titleStyle.Render("Add Gate"))
	sb.WriteString("\n")

	// Category tabs
	for i, cat := range gateMenu {
		name := " " + cat.name + " "
		if i == m.menuCat {
			sb.WriteString(activeGateStyle.Render(name))
		} else {
			sb.WriteString(dimStyle.Render(name))
		}
		if i < len(gateMenu)-1 {
			sb.WriteString(dimStyle.Render("│"))
		}
	}
	sb.WriteString("\n")
	sb.WriteString(dimStyle.Render(strings.Repeat("─", 42)))
	sb.WriteString("\n")

	// Items in the selected category
	cat := gateMenu[m.menuCat]
	for i, item := range cat.items {
		if i == m.menuItem {
			sb.WriteString(menuSelectedStyle.Render(" ▸ "))
			sb.WriteString(menuSelectedStyle.Render(fmt.Sprintf("%-18s", item.name)))
			sb.WriteString(gateStyle.Render(item.symbol))
		} else {
			sb.WriteString("   ")
			sb.WriteString(menuNormalStyle.Render(fmt.Sprintf("%-18s", item.name)))
			sb.WriteString(dimStyle.Render(item.symbol))
		}
		if item.needsTarget {
			sb.WriteString(dimStyle.Render(" →target"))
		}
		if item.needsParams {
			sb.WriteString(dimStyle.Render(fmt.Sprintf(" (%s)", item.paramHint.example)))
		}
		sb.WriteString("\n")
	}
	sb.WriteString(dimStyle.Render(" ↑↓ Select  ←→ Cat  ⏎ Ok  Esc ✕"))

	return menuBorderStyle.Render(sb.String())
}

// isParameterizedGate returns true if the gate type requires parameters
func isParameterizedGate(gateType string) bool {
	parameterizedGates := map[string]bool{
		"RX": true, "RY": true, "RZ": true,
		"P": true, "U1": true, "U2": true, "U3": true,
		"CRX": true, "CRY": true, "CRZ": true, "CU1": true,
		"NOISE_DEPOL": true, "NOISE_AMP": true, "NOISE_PHASE": true,
	}
	return parameterizedGates[gateType]
}
