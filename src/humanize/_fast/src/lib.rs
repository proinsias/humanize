use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PySequence};
use rayon::prelude::*;
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

/// Format a single value into intcomma string
fn intcomma_single(value: &PyAny, ndigits: Option<usize>) -> PyResult<String> {
    let val_str: String = if let Ok(s) = value.extract::<String>() {
        s
    } else if let Ok(f) = value.extract::<f64>() {
        f.to_string()
    } else if value.is_none() {
        return Ok("None".to_string());
    } else {
        return Ok(format!("{:?}", value));
    };

    let value_num = match parse_value(&val_str) {
        Some(v) => v,
        None => return Ok(val_str),
    };

    if !value_num.is_finite() {
        return Ok(format_not_finite(value_num));
    }

    let orig = if let Some(d) = ndigits {
        format!("{:.1$}", value_num, d)
    } else {
        value_num.to_string()
    };

    let parts: Vec<&str> = orig.split('.').collect();
    let mut whole = parts[0].to_string();
    let fraction = if parts.len() > 1 { Some(parts[1]) } else { None };

    let re = Regex::new(r"^(-?\d+)(\d{3})").unwrap();
    loop {
        let new = re.replace(&whole, "$1,$2").to_string();
        if new == whole {
            break;
        }
        whole = new;
    }

    let mut result = String::new();
    write!(&mut result, "{}", whole).unwrap();
    if let Some(f) = fraction {
        write!(&mut result, ".{}", f).unwrap();
    }

    Ok(result)
}

/// Recursive function to handle nested iterables
fn intcomma_recursive(py: Python<'_>, value: &PyAny, ndigits: Option<usize>) -> PyResult<PyObject> {
    if let Ok(seq) = value.downcast::<PySequence>() {
        let len = seq.len()? as usize;
        // Use Rayon for parallel processing
        let items: Vec<PyObject> = (0..len)
            .into_par_iter()
            .map(|i| {
                let item = seq.get_item(i).unwrap();
                intcomma_recursive(py, item, ndigits).unwrap()
            })
            .collect();
        Ok(items.into_py(py))
    } else {
        intcomma_single(value, ndigits).map(|s| s.into_py(py))
    }
}

/// PyO3 wrapper
#[pyfunction(signature = (value, ndigits=None))]
fn intcomma(py: Python<'_>, value: &PyAny, ndigits: Option<usize>) -> PyResult<PyObject> {
    intcomma_recursive(py, value, ndigits)
}

/// PyO3 module definition
#[pymodule]
fn _fast(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(intcomma, m)?)?;
    Ok(())
}

// FIXME: Add unit tests.
// FIXME: Add benchmarks.
