use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple, PyAny};
use pyo3::{Bound, Python};
use rayon::prelude::*;
use std::fmt::Write;

// --- Helper Functions (Unchanged) ---

/// Helper: safely parse a value that could be int, float, or string.
fn parse_value(value: &str) -> Option<f64> {
    // We only remove commas to handle common thousands separators.
    let cleaned = value.replace(',', "");
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

/// Helper: insert commas every 3 digits in the whole part
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
    // 1. Parse numeric value
    let value_num = match parse_value(&val_str) {
        Some(v) => v,
        None => return val_str, // Return original string if not numeric
    };

    // 2. Handle non-finite
    if !value_num.is_finite() {
        return format_not_finite(value_num);
    }

    // 3. Format with optional ndigits
    let orig = if let Some(d) = ndigits {
        format!("{:.1$}", value_num, d)
    } else {
        value_num.to_string()
    };

    // 4. Split and apply commas
    let parts: Vec<&str> = orig.split('.').collect();
    // Safely handle negative sign if present
    let (whole_str, is_negative) = if parts[0].starts_with('-') {
        (&parts[0][1..], true)
    } else {
        (parts[0], false)
    };

    let mut whole = add_commas(whole_str);
    if is_negative {
        whole.insert(0, '-');
    }

    let fraction = if parts.len() > 1 { Some(parts[1]) } else { None };

    // 5. Build final result
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
#[pyfunction(signature = (value, ndigits=None))]
fn intcomma(py: Python<'_>, value: &Bound<'_, PyAny>, ndigits: Option<usize>) -> PyResult<PyObject> {

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
            // If repr() fails (returns Err), fall back to a basic string representation.
            val.repr()
               .map(|s| s.to_string())
               .unwrap_or_else(|_| format!("<unprintable object of type {}>", val.get_type()))
        }
    };

    // Attempt to downcast as a PyList
    if let Ok(iterable) = value.downcast::<PyList>() {
        // FIX: Borrow the val (&val) here
        let string_values: Vec<String> = iterable.iter().map(|val| element_to_string(&val)).collect();

        // Parallel processing (GIL not required)
        let results: Vec<String> = string_values
            .into_par_iter()
            .map(|val_str| format_single_value(val_str, ndigits))
            .collect();

        // Convert the result back to a Python List
        return Ok(results.to_object(py));
    }

    // Attempt to downcast as a PyTuple
    if let Ok(iterable) = value.downcast::<PyTuple>() {
        // FIX: Borrow the val (&val) here
        let string_values: Vec<String> = iterable.iter().map(|val| element_to_string(&val)).collect();

        let results: Vec<String> = string_values
            .into_par_iter()
            .map(|val_str| format_single_value(val_str, ndigits))
            .collect();

        // Convert the result back to a Python Tuple
        return Ok(PyTuple::new_bound(py, results).to_object(py));
    }

    // If not an iterable (list/tuple), treat it as a single value

    let val_str = if let Ok(s) = value.extract::<String>() {
        s
    } else if let Ok(f) = value.extract::<f64>() {
        f.to_string()
    } else if let Ok(i) = value.extract::<i64>() {
        i.to_string()
    } else if value.is_none() {
        return Ok("None".to_object(py));
    } else {
        // Handle unhandled non-iterable types using PyAny::repr()
        let repr_result = value.repr().map(|s| s.to_string());
        return Ok(repr_result.unwrap_or_else(|_| format!("<unprintable object of type {}>", value.get_type())).to_object(py));
    };

    let result = format_single_value(val_str, ndigits);

    // Return the single result as a Python String
    Ok(result.to_object(py))
}

// --- PyO3 Module Definition (Fixed) ---

/// PyO3 module definition
#[pymodule]
fn _fast(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Using the non-deprecated wrap_pyfunction_bound!
    m.add_function(wrap_pyfunction_bound!(intcomma, m)?)?;
    Ok(())
}

