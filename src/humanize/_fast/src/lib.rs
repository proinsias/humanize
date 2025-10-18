use pyo3::prelude::*;
use regex::Regex;
use std::fmt::Write;

/// Helper: safely parse a value that could be int, float, or string.
fn parse_value(value: &str) -> Option<f64> {
    let cleaned = value.replace(',', "").replace('.', ".");
    cleaned.parse::<f64>().ok()
}

/// Helper: format non-finite values (inf, -inf, nan)
fn format_not_finite(v: f64) -> String {
    if v.is_nan() {
        "NaN".to_string()
    } else if v.is_infinite() {
        if v.is_sign_positive() {
            "inf".to_string()
        } else {
            "-inf".to_string()
        }
    } else {
        v.to_string()
    }
}

/// Rust version of humanize.intcomma
#[pyfunction(signature = (value, ndigits=None))]
fn intcomma(py: Python<'_>, value: PyObject, ndigits: Option<usize>) -> PyResult<String> {
    // Python values can be many types, so we handle via PyAny introspection.
    let val_str = if let Ok(s) = value.extract::<String>(py) {
        s
    } else if let Ok(f) = value.extract::<f64>(py) {
        f.to_string()
    } else if value.is_none(py) {
        return Ok("None".to_string());
    } else {
        // Try to cast to string as fallback
        return Ok(format!("{:?}", value));
    };

    // Attempt to parse numeric value
    let maybe_val = parse_value(&val_str);

    let value_num = match maybe_val {
        Some(v) => v,
        None => return Ok(val_str),
    };

    // Handle non-finite numbers
    if !value_num.is_finite() {
        return Ok(format_not_finite(value_num));
    }

    // Format with specified precision (if any)
    let orig = if let Some(d) = ndigits {
        format!("{:.1$}", value_num, d)
    } else {
        value_num.to_string()
    };

    // Split into whole and fractional parts
    let parts: Vec<&str> = orig.split('.').collect();
    let mut whole = parts[0].to_string();
    let fraction = if parts.len() > 1 { Some(parts[1]) } else { None };

    // Insert commas every 3 digits in the whole part
    let re = Regex::new(r"^(-?\d+)(\d{3})").unwrap();
    loop {
        let new = re.replace(&whole, "$1,$2").to_string();
        if new == whole {
            break;
        }
        whole = new;
    }

    // Reassemble
    let mut result = String::new();
    write!(&mut result, "{}", whole).unwrap();
    if let Some(f) = fraction {
        write!(&mut result, ".{}", f).unwrap();
    }

    Ok(result)
}

/// PyO3 module definition
#[pymodule]
fn _fast(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(intcomma, m)?)?;
    Ok(())
}

// FIXME: Add benchmarks.
// FIXME: Add pre-commit.
