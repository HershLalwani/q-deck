package main

import (
	"fmt"
	"strings"
)

// menuItem represents a single gate choice in the menu.
type menuItem struct {
	name        string
	gateType    string
	symbol      string
	needsTarget bool
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
			{name: "Phase (S)", gateType: "S", symbol: "S"},
			{name: "T Gate", gateType: "T", symbol: "T"},
		},
	},
	{
		name: "Multi Qubit",
		items: []menuItem{
			{name: "CNOT", gateType: "CX", symbol: "●─⊕", needsTarget: true},
			{name: "Controlled-Z", gateType: "CZ", symbol: "●─●", needsTarget: true},
			{name: "SWAP", gateType: "SWAP", symbol: "×─×", needsTarget: true},
		},
	},
	{
		name: "Measurement",
		items: []menuItem{
			{name: "Measure", gateType: "MEASURE", symbol: "M"},
			{name: "Measure-Ctrl X", gateType: "MCX", symbol: "M─⊕", needsTarget: true},
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
		sb.WriteString("\n")
	}
	sb.WriteString(dimStyle.Render(" ↑↓ Select  ←→ Cat  ⏎ Ok  Esc ✕"))

	return menuBorderStyle.Render(sb.String())
}
