//! filesize.rs
//!
//! naturalsize implementation, uses format_utils.rs

use crate::format_utils::*;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyList, PyString, PyTuple};
use pyo3::{Bound, Python};
use rayon::prelude::*;

/// Intermediate extracted value
enum Extracted {
    Numeric(f64),
    Raw(String),
}

/// Convert PyAny to Extracted
fn extract_to_extracted(obj: &Bound<'_, PyAny>) -> Extracted {
    if let Ok(f) = obj.extract::<f64>() {
        Extracted::Numeric(f)
    } else if let Ok(i) = obj.extract::<i64>() {
        Extracted::Numeric(i as f64)
    } else if let Ok(s) = obj.extract::<String>() {
        if let Some(parsed) = parse_value(&s) {
            Extracted::Numeric(parsed)
        } else {
            Extracted::Raw(s)
        }
    } else if obj.is_none() {
        Extracted::Raw("None".to_string())
    } else {
        match obj.repr() {
            Ok(s) => Extracted::Raw(s.to_string()),
            Err(_) => Extracted::Raw(format!("<unprintable object of type {}>", obj.get_type())),
        }
    }
}

/// Format numeric value according to decimal/binary/gnu rules
fn format_numeric_value(bytes: f64, binary: bool, gnu: bool, format_spec: &str) -> String {
    let base = if gnu || binary { 1024f64 } else { 1000f64 };
    let abs_bytes = bytes.abs();

    if abs_bytes == 1.0 && !gnu {
        return format!("{} Byte", bytes as i64);
    }

    if abs_bytes < base {
        if gnu {
            return format!("{}B", bytes as i64);
        } else {
            return format!("{} Bytes", bytes as i64);
        }
    }

    if gnu {
        let exp_f = (abs_bytes.ln() / base.ln()).floor();
        let mut exp = exp_f as i32;
        let max_exp = SUFFIXES_GNU.len() as i32;
        if exp > max_exp {
            exp = max_exp;
        }
        if exp < 1 {
            exp = 1;
        }
        let divisor = base.powi(exp);
        let value = bytes / divisor;
        let formatted = apply_printf_style(format_spec, value);
        let ch = SUFFIXES_GNU.chars().nth((exp - 1) as usize).unwrap_or('?');
        format!("{}{}", formatted, ch)
    } else {
        let suffix = if binary {
            &SUFFIXES_BINARY[..]
        } else {
            &SUFFIXES_DECIMAL[..]
        };
        let log_val = (abs_bytes.ln() / base.ln()).floor();
        let mut exp = log_val as usize;
        if exp > suffix.len() {
            exp = suffix.len();
        }
        if exp < 1 {
            exp = 1;
        }
        let divisor = base.powi(exp as i32);
        let value = bytes / divisor;
        let formatted = apply_printf_style(format_spec, value);
        let suf = suffix[exp - 1];
        format!("{}{}", formatted, suf)
    }
}

/// PyO3-exposed naturalsize
#[pyfunction(signature = (value, binary=false, gnu=false, format="%.1f"))]
pub fn naturalsize(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
    binary: bool,
    gnu: bool,
    format: &str,
) -> PyResult<PyObject> {
    let is_string = value.is_instance_of::<PyString>() || value.is_instance_of::<PyBytes>();

    if !is_string {
        if let Ok(iterator) = value.iter() {
            let mut extracted_items = Vec::new();
            for maybe_item in iterator {
                match maybe_item {
                    Ok(item) => extracted_items.push(extract_to_extracted(&item)),
                    Err(_) => {
                        return Err(pyo3::exceptions::PyRuntimeError::new_err(
                            "error iterating value",
                        ))
                    }
                }
            }

            let results: Vec<String> = extracted_items
                .into_par_iter()
                .map(|ex| match ex {
                    Extracted::Numeric(n) => {
                        if !n.is_finite() {
                            format_not_finite(n)
                        } else {
                            format_numeric_value(n, binary, gnu, format)
                        }
                    }
                    Extracted::Raw(s) => normalize_special_values(&s).unwrap_or(s),
                })
                .collect();

            if value.is_instance_of::<PyList>() {
                return Ok(PyList::new_bound(py, results).to_object(py));
            } else if value.is_instance_of::<PyTuple>() {
                return Ok(PyTuple::new_bound(py, results).to_object(py));
            } else {
                return Ok(PyList::new_bound(py, results).to_object(py));
            }
        }
    }

    // scalar path
    if let Ok(f) = value.extract::<f64>() {
        if !f.is_finite() {
            return Ok(format_not_finite(f).to_object(py));
        }
        return Ok(format_numeric_value(f, binary, gnu, format).to_object(py));
    }
    if let Ok(i) = value.extract::<i64>() {
        return Ok(format_numeric_value(i as f64, binary, gnu, format).to_object(py));
    }
    if let Ok(s) = value.extract::<String>() {
        if let Some(norm) = normalize_special_values(&s) {
            return Ok(norm.to_object(py));
        }
        if let Some(parsed) = parse_value(&s) {
            if !parsed.is_finite() {
                return Ok(format_not_finite(parsed).to_object(py));
            }
            return Ok(format_numeric_value(parsed, binary, gnu, format).to_object(py));
        }
        return Ok(s.to_object(py));
    }
    if value.is_none() {
        return Ok("None".to_object(py));
    }

    let repr_result = value.repr().map(|s| s.to_string());
    Ok(repr_result
        .unwrap_or_else(|_| format!("<unprintable object of type {}>", value.get_type()))
        .to_object(py))
}
