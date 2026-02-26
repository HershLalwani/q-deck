use regex::Regex;
use std::f64::consts::PI;
use std::sync::OnceLock;

static PI_EXPR_REGEX: OnceLock<Regex> = OnceLock::new();

pub fn parse_param_expr(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if let Ok(val) = s.parse::<f64>() {
        return Some(val);
    }

    let s = s.to_lowercase();
    let re = PI_EXPR_REGEX.get_or_init(|| {
        Regex::new(r"^(-?)(\d*\.?\d*)\s*\*?\s*pi(?:\s*/\s*(\d+\.?\d*))?$").unwrap()
    });

    if let Some(caps) = re.captures(&s) {
        let negative = &caps[1] == "-";
        let coeff_str = &caps[2];
        let denom_str = caps.get(3).map_or("", |m| m.as_str());

        let mut coeff = 1.0;
        if !coeff_str.is_empty() {
            coeff = coeff_str.parse::<f64>().ok()?;
        }

        let mut result = coeff * PI;

        if !denom_str.is_empty() {
            let denom = denom_str.parse::<f64>().ok()?;
            if denom == 0.0 {
                return None;
            }
            result /= denom;
        }

        if negative {
            result = -result;
        }
        return Some(result);
    }

    None
}

pub fn format_param(val: f64) -> String {
    struct PiForm {
        value: f64,
        display: &'static str,
    }

    let pi_forms = [
        PiForm {
            value: 2.0 * PI,
            display: "2*pi",
        },
        PiForm {
            value: PI,
            display: "pi",
        },
        PiForm {
            value: PI / 2.0,
            display: "pi/2",
        },
        PiForm {
            value: PI / 3.0,
            display: "pi/3",
        },
        PiForm {
            value: PI / 4.0,
            display: "pi/4",
        },
        PiForm {
            value: PI / 6.0,
            display: "pi/6",
        },
        PiForm {
            value: PI / 8.0,
            display: "pi/8",
        },
        PiForm {
            value: 3.0 * PI / 4.0,
            display: "3*pi/4",
        },
        PiForm {
            value: 3.0 * PI / 2.0,
            display: "3*pi/2",
        },
        PiForm {
            value: 2.0 * PI / 3.0,
            display: "2*pi/3",
        },
    ];

    for pf in &pi_forms {
        if (val - pf.value).abs() < 1e-10 {
            return pf.display.to_string();
        }
        if (val + pf.value).abs() < 1e-10 {
            return format!("-{}", pf.display);
        }
    }

    val.to_string()
}

pub fn parse_params(input: &str) -> Option<Vec<f64>> {
    let mut params = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(val) = parse_param_expr(part) {
            params.push(val);
        } else {
            return None; // validation failure
        }
    }
    Some(params)
}
