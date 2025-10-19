use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyTuple};
use pyo3::{Bound, Python};
use rayon::prelude::*;
use std::fmt::Write;

use crate::format_utils::{
    add_commas, apply_printf_style, format_not_finite, normalize_special_values, parse_value,
};

const POWERS: [f64; 12] = [
    1e3, 1e6, 1e9, 1e12, 1e15, 1e18, 1e21, 1e24, 1e27, 1e30, 1e33, 1e100,
];

const HUMAN_POWERS: [&str; 12] = [
    "thousand",
    "million",
    "billion",
    "trillion",
    "quadrillion",
    "quintillion",
    "sextillion",
    "septillion",
    "octillion",
    "nonillion",
    "decillion",
    "googol",
];

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

fn intword_single(val_str: String, format_spec: &str) -> String {
    // Normalize special float strings
    if let Some(normalized) = normalize_special_values(&val_str) {
        return normalized;
    }

    // Attempt to parse
    let value_num = match parse_value(&val_str) {
        Some(v) => v,
        None => return val_str,
    };

    // Handle NaN/Inf
    if !value_num.is_finite() {
        return format_not_finite(value_num);
    }

    // Negative handling
    let mut value = value_num;
    let mut negative_prefix = String::new();
    if value < 0.0 {
        negative_prefix.push('-');
        value = -value;
    }

    // If smaller than 1,000 → return raw
    if value < POWERS[0] {
        return format!(
            "{}{}",
            negative_prefix,
            add_commas(&value.trunc().to_string())
        );
    }

    // Find appropriate power
    for (i, power) in POWERS.iter().enumerate().skip(1) {
        if value < *power {
            let chopped = value / POWERS[i - 1];
            let powers_diff = POWERS[i] / POWERS[i - 1];
            let formatted = apply_printf_style(format_spec, chopped);
            let formatted_f = parse_value(&formatted).unwrap_or(chopped);

            // Detect if rounding overflows (e.g., "1000.0 thousand" → "1.0 million")
            if (formatted_f - powers_diff).abs() < f64::EPSILON {
                let chopped2 = value / POWERS[i];
                let formatted2 = apply_printf_style(format_spec, chopped2);
                return format!("{}{} {}", negative_prefix, formatted2, HUMAN_POWERS[i]);
            }

            return format!("{}{} {}", negative_prefix, formatted, HUMAN_POWERS[i - 1]);
        }
    }

    // Beyond googol — return the raw number
    format!("{}{}", negative_prefix, value)
}

/// Rust version of `humanize.intword`
///
/// Examples
/// --------
/// >>> import _fast
/// >>> _fast.intword("100")
/// '100'
/// >>> _fast.intword("12400")
/// '12.4 thousand'
/// >>> _fast.intword("1000000")
/// '1.0 million'
/// >>> _fast.intword(1_200_000_000)
/// '1.2 billion'
/// >>> _fast.intword(8100000000000000000000000000000000)
/// '8.1 decillion'
/// >>> _fast.intword(None)
/// 'None'
/// >>> _fast.intword("1234000", "%0.3f")
/// '1.234 million'
/// >>> _fast.intword([100, 12400, "1000000"])
/// ['100', '12.4 thousand', '1.0 million']
#[pyfunction(signature = (value, format="%.1f"))]
pub fn intword(py: Python<'_>, value: &Bound<'_, PyAny>, format: &str) -> PyResult<PyObject> {
    // Convert scalar or iterable, parallelize if possible
    if let Ok(iterable) = value.downcast::<PyList>() {
        let string_values: Vec<String> = iterable
            .iter()
            .map(|val| val.str().unwrap().to_string())
            .collect();
        let results: Vec<String> = string_values
            .into_par_iter()
            .map(|val| intword_single(val, format))
            .collect();
        return Ok(results.to_object(py));
    }

    if let Ok(iterable) = value.downcast::<PyTuple>() {
        let string_values: Vec<String> = iterable
            .iter()
            .map(|val| val.str().unwrap().to_string())
            .collect();
        let results: Vec<String> = string_values
            .into_par_iter()
            .map(|val| intword_single(val, format))
            .collect();
        return Ok(PyTuple::new_bound(py, results).to_object(py));
    }

    // Scalar handling
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

    let result = intword_single(val_str, format);
    Ok(result.to_object(py))
}
