package main

import "github.com/charmbracelet/lipgloss"

// Layout constants
const (
	cellW        = 11 // width of each step column in characters
	labelVisualW = 7  // visual width of qubit label area
	gateNameW    = 5  // width of gate name inside box
	gateBoxW     = 7  // ┤ + gateNameW + ├ = 1 + 5 + 1
)

// Lipgloss styles used across the TUI.
var (
	circuitStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("#7aa2f7")).
			Padding(1)

	qasmStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("#bb9af7")).
			Padding(1)

	controlsStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("#9ece6a")).
			Padding(0, 1)

	titleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#ff9e64"))

	cursorBoxStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#ff9e64")).
			Bold(true)

	targetSelectStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#bb9af7")).
				Bold(true)

	activeGateStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#e0af68"))

	qubitLabelStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#7dcfff"))

	gateStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#73daca"))

	dimStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#565f89"))

	menuBorderStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("#ff9e64")).
			Padding(0, 1)

	menuSelectedStyle = lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#ff9e64"))

	menuNormalStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#c0caf5"))

	cbitLabelStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#e0af68"))

	cbitWireStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#565f89"))

	cbitConnectorStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#e0af68")).
				Bold(true)
)
