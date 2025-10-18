use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyTuple};
use pyo3::{Bound, Python};
use rayon::prelude::*;
use std::fmt::Write;

// --- Helper Functions ---

/// Helper: safely parse a value that could be int, float, or string.
fn parse_value(value: &str) -> Option<f64> {
    // We only remove commas to handle common thousands separators.
    let cleaned = value.replace(',', "");
    cleaned.parse::<f64>().ok()
}

/// Normalize string representations of special float values to Python-style capitalization.
/// ("inf" -> "Inf", "-inf" -> "-Inf", "nan" -> "NaN")
fn normalize_special_values(s: &str) -> Option<String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "inf" | "+inf" => Some("+Inf".to_string()),
        "-inf" => Some("-Inf".to_string()),
        "nan" => Some("NaN".to_string()),
        _ => None,
    }
}

/// Helper: format non-finite values (Inf, -Inf, NaN) to Python-style capitalization.
fn format_not_finite(v: f64) -> String {
    if v.is_nan() {
        "NaN".to_string()
    } else if v.is_infinite() {
        if v.is_sign_positive() {
            "Inf".to_string() // Python-style
        } else {
            "-Inf".to_string() // Python-style
        }
    } else {
        v.to_string()
    }
}

/// Helper: insert commas every 3 digits in the whole part.
fn add_commas(whole: &str) -> String {
    let chars: Vec<char> = whole.chars().collect();
    let mut result = String::new();
    let mut count = 0;
    for &c in chars.iter().rev() {
        if count == 3 {
            result.push(',');
            count = 0;
        }
        result.push(c);
        count += 1;
    }
    result.chars().rev().collect()
}

// --- Core Formatting Logic ---

/// Core logic for formatting a single string value.
fn format_single_value(val_str: String, ndigits: Option<usize>) -> String {
    // 1. Normalize explicitly stringified "inf", "-inf", "nan"
    if let Some(normalized) = normalize_special_values(&val_str) {
        return normalized;
    }

    // 2. Parse numeric value
    let value_num = match parse_value(&val_str) {
        Some(v) => v,
        None => return val_str, // Return original string if not numeric
    };

    // 3. Handle non-finite
    if !value_num.is_finite() {
        return format_not_finite(value_num);
    }

    // 4. Format with optional ndigits
    let orig = if let Some(d) = ndigits {
        format!("{:.1$}", value_num, d)
    } else {
        value_num.to_string()
    };

    // 5. Split and apply commas
    let parts: Vec<&str> = orig.split('.').collect();
    // Handle negative sign if present
    let (whole_str, is_negative) = if parts[0].starts_with('-') {
        (&parts[0][1..], true)
    } else {
        (parts[0], false)
    };

    let mut whole = add_commas(whole_str);
    if is_negative {
        whole.insert(0, '-');
    }

    let fraction = if parts.len() > 1 {
        Some(parts[1])
    } else {
        None
    };

    // 6. Build final result
    let mut result = String::new();
    write!(&mut result, "{}", whole).unwrap();
    if let Some(f) = fraction {
        write!(&mut result, ".{}", f).unwrap();
    }

    result
}

// --- Unified PyO3 Function ---

/// Rust version of humanize.intcomma for a single value (str, float, int)
/// or an iterable (list, tuple) of those types.
///
/// Examples
/// --------
/// >>> import _fast
/// >>> _fast.intcomma("1234567")
/// '1,234,567'
/// >>> _fast.intcomma("-inf")
/// '-Inf'
/// >>> _fast.intcomma(["1234567", "-inf", "nan"])
/// ['1,234,567', '-Inf', 'NaN']
#[pyfunction(signature = (value, ndigits=None))]
fn intcomma(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    ndigits: Option<usize>,
) -> PyResult<PyObject> {
    // Helper closure to convert any element in the iterable to a Rust String
    let element_to_string = |val: &Bound<'_, PyAny>| -> String {
        if let Ok(s) = val.extract::<String>() {
            s
        } else if let Ok(f) = val.extract::<f64>() {
            f.to_string()
        } else if let Ok(i) = val.extract::<i64>() {
            i.to_string()
        } else if val.is_none() {
            "None".to_string()
        } else {
            val.repr()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("<unprintable object of type {}>", val.get_type()))
        }
    };

    // Handle list input
    if let Ok(iterable) = value.downcast::<PyList>() {
        let string_values: Vec<String> =
            iterable.iter().map(|val| element_to_string(&val)).collect();
        let results: Vec<String> = string_values
            .into_par_iter()
            .map(|val_str| format_single_value(val_str, ndigits))
            .collect();
        return Ok(results.to_object(py));
    }

    // Handle tuple input
    if let Ok(iterable) = value.downcast::<PyTuple>() {
        let string_values: Vec<String> =
            iterable.iter().map(|val| element_to_string(&val)).collect();
        let results: Vec<String> = string_values
            .into_par_iter()
            .map(|val_str| format_single_value(val_str, ndigits))
            .collect();
        return Ok(PyTuple::new_bound(py, results).to_object(py));
    }

    // Handle scalar (single value)
    let val_str = if let Ok(s) = value.extract::<String>() {
        s
    } else if let Ok(f) = value.extract::<f64>() {
        f.to_string()
    } else if let Ok(i) = value.extract::<i64>() {
        i.to_string()
    } else if value.is_none() {
        return Ok("None".to_object(py));
    } else {
        let repr_result = value.repr().map(|s| s.to_string());
        return Ok(repr_result
            .unwrap_or_else(|_| format!("<unprintable object of type {}>", value.get_type()))
            .to_object(py));
    };

    let result = format_single_value(val_str, ndigits);
    Ok(result.to_object(py))
}

// --- PyO3 Module Definition (Fixed) ---

/// PyO3 module definition
#[pymodule]
fn _fast(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction_bound!(intcomma, m)?)?;
    Ok(())
}
