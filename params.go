package main

import (
	"fmt"
	"math"
	"regexp"
	"strconv"
	"strings"
)

// paramPattern matches a single parameter value: numbers, pi expressions, or combinations.
// Examples: "1.5707", "pi", "pi/2", "3*pi/4", "-pi", "-2*pi/3", "3.14e-2"
const paramPattern = `-?(?:\d*\.?\d*\*?pi(?:/\d+\.?\d*)?|\d+\.?\d*(?:[eE][+\-]?\d+)?)`

// piExprRegex matches expressions like: pi, 2pi, 2*pi, pi/2, 3pi/4, 3*pi/4, -pi, -pi/2, -3*pi/4
var piExprRegex = regexp.MustCompile(`^(-?)(\d*\.?\d*)\s*\*?\s*pi(?:\s*/\s*(\d+\.?\d*))?$`)

// parseParamExpr parses a single parameter expression, supporting plain numbers and pi expressions.
// Returns the parsed float64 value and true on success, or 0 and false on failure.
//
// Supported formats:
//   - Plain numbers: "1.5707", "3.14", "-0.5"
//   - Pi constant: "pi"
//   - Pi fractions: "pi/2", "pi/4", "pi/3"
//   - Coefficients: "2pi", "2*pi", "3pi/4", "3*pi/4"
//   - Negative: "-pi", "-pi/2", "-3*pi/4"
func parseParamExpr(s string) (float64, bool) {
	s = strings.TrimSpace(s)
	if s == "" {
		return 0, false
	}

	// Try plain number first
	if val, err := strconv.ParseFloat(s, 64); err == nil {
		return val, true
	}

	// Try pi expression
	s = strings.ToLower(s)
	if matches := piExprRegex.FindStringSubmatch(s); matches != nil {
		negative := matches[1] == "-"
		coeffStr := matches[2]
		denomStr := matches[3]

		coeff := 1.0
		if coeffStr != "" {
			var err error
			coeff, err = strconv.ParseFloat(coeffStr, 64)
			if err != nil {
				return 0, false
			}
		}

		result := coeff * math.Pi

		if denomStr != "" {
			denom, err := strconv.ParseFloat(denomStr, 64)
			if err != nil || denom == 0 {
				return 0, false
			}
			result /= denom
		}

		if negative {
			result = -result
		}
		return result, true
	}

	return 0, false
}

// formatParam formats a float64 parameter value, using pi notation when possible.
// Recognizes common pi fractions: pi, pi/2, pi/4, pi/3, pi/6, pi/8, 2pi, 3pi/4, etc.
func formatParam(val float64) string {
	// Table of recognized pi fractions: coefficient, denominator, display string
	type piForm struct {
		value   float64
		display string
	}
	piForms := []piForm{
		{2 * math.Pi, "2*pi"},
		{math.Pi, "pi"},
		{math.Pi / 2, "pi/2"},
		{math.Pi / 3, "pi/3"},
		{math.Pi / 4, "pi/4"},
		{math.Pi / 6, "pi/6"},
		{math.Pi / 8, "pi/8"},
		{3 * math.Pi / 4, "3*pi/4"},
		{3 * math.Pi / 2, "3*pi/2"},
		{2 * math.Pi / 3, "2*pi/3"},
	}

	for _, pf := range piForms {
		if math.Abs(val-pf.value) < 1e-10 {
			return pf.display
		}
		if math.Abs(val+pf.value) < 1e-10 {
			return "-" + pf.display
		}
	}

	return fmt.Sprintf("%g", val)
}

// parseParams parses a parameter string into float values.
// Returns nil if any part fails to parse.
func (m *Model) parseParams(input string) []float64 {
	var params []float64
	parts := strings.Split(input, ",")
	for _, part := range parts {
		part = strings.TrimSpace(part)
		if part == "" {
			continue
		}
		val, ok := parseParamExpr(part)
		if !ok {
			return nil // validation failure
		}
		params = append(params, val)
	}
	return params
}
