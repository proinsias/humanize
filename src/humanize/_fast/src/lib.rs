use pyo3::prelude::*;
use rayon::prelude::*;
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

/// Rust version of humanize.intcomma for single value
#[pyfunction(signature = (value, ndigits=None))]
fn intcomma(py: Python<'_>, value: PyObject, ndigits: Option<usize>) -> PyResult<String> {
    // Extract Python object to string
    let val_str = if let Ok(s) = value.extract::<String>(py) {
        s
    } else if let Ok(f) = value.extract::<f64>(py) {
        f.to_string()
    } else if value.is_none(py) {
        return Ok("None".to_string());
    } else {
        return Ok(format!("{:?}", value));
    };

    // Parse numeric value
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

    whole = add_commas(&whole);

    let mut result = String::new();
    write!(&mut result, "{}", whole).unwrap();
    if let Some(f) = fraction {
        write!(&mut result, ".{}", f).unwrap();
    }

    Ok(result)
}

/// Rust version of humanize.intcomma for a list of Python objects (mixed types) using Rayon
#[pyfunction(signature = (values, ndigits=None))]
fn intcomma_vec(py: Python<'_>, values: Vec<PyObject>, ndigits: Option<usize>) -> PyResult<Vec<String>> {
    // 1️⃣ Extract Python objects to Rust strings (GIL required)
    let string_values: Vec<String> = values
        .iter()
        .map(|val| {
            if let Ok(s) = val.extract::<String>(py) {
                s
            } else if let Ok(f) = val.extract::<f64>(py) {
                f.to_string()
            } else if val.is_none(py) {
                "None".to_string()
            } else {
                format!("{:?}", val)
            }
        })
        .collect();

    // 2️⃣ Parallel processing on pure Rust strings
    let results: Vec<String> = string_values
        .into_par_iter()
        .map(|val_str| {
            let value_num = match parse_value(&val_str) {
                Some(v) => v,
                None => return val_str,
            };

            if !value_num.is_finite() {
                return format_not_finite(value_num);
            }

            let orig = if let Some(d) = ndigits {
                format!("{:.1$}", value_num, d)
            } else {
                value_num.to_string()
            };

            let parts: Vec<&str> = orig.split('.').collect();
            let mut whole = parts[0].to_string();
            let fraction = if parts.len() > 1 { Some(parts[1]) } else { None };

            whole = add_commas(&whole);

            let mut result = String::new();
            write!(&mut result, "{}", whole).unwrap();
            if let Some(f) = fraction {
                write!(&mut result, ".{}", f).unwrap();
            }

            result
        })
        .collect();

    Ok(results)
}

/// PyO3 module definition
#[pymodule]
fn _fast(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(intcomma, m.py())?)?;
    m.add_function(wrap_pyfunction!(intcomma_vec, m.py())?)?;
    Ok(())
}

// FIXME: Add pre-commit.
// FIXME: Get support for iterables in python and rust working. !!!Use latest gpt!!!
// FIXME: Add tests for iterable support in python.
