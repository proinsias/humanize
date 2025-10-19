use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyTuple};
use pyo3::{Bound, Python};
use rayon::prelude::*;
use std::fmt::Write;

use crate::format_utils::{add_commas, format_not_finite, normalize_special_values, parse_value};

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
pub fn intcomma(
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
