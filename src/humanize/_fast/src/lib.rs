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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intcomma_integers() {
        Python::with_gil(|py| {
            // Simple integer
            let val = 1000i64.to_object(py);
            let result = intcomma(py, val, None).unwrap();
            assert_eq!(result, "1,000");

            // Large integer
            let val = 1234567890i64.to_object(py);
            let result = intcomma(py, val, None).unwrap();
            assert_eq!(result, "1,234,567,890");
        });
    }

    #[test]
    fn test_intcomma_floats() {
        Python::with_gil(|py| {
            // Float with default formatting
            let val = 12345.6789f64.to_object(py);
            let result = intcomma(py, val, None).unwrap();
            assert_eq!(result, "12,345.6789");

            // Float with rounding
            let val = 12345.6789f64.to_object(py);
            let result = intcomma(py, val, Some(2)).unwrap();
            assert_eq!(result, "12,345.68");
        });
    }

    #[test]
    fn test_intcomma_numeric_string() {
        Python::with_gil(|py| {
            // String containing number
            let val = "9876543.21".to_object(py);
            let result = intcomma(py, val, Some(1)).unwrap();
            assert_eq!(result, "9,876,543.2");
        });
    }

    #[test]
    fn test_intcomma_invalid_input() {
        Python::with_gil(|py| {
            let val = "not_a_number".to_object(py);
            let err = intcomma(py, val, None).unwrap_err();
            assert!(err.is_instance_of::<pyo3::exceptions::PyValueError>(py));
        });
    }

    #[test]
    fn test_intcomma_negative_numbers() {
        Python::with_gil(|py| {
            let val = (-1234567i64).to_object(py);
            let result = intcomma(py, val, None).unwrap();
            assert_eq!(result, "-1,234,567");
        });
    }
}

#[cfg(test)]
mod rayon_tests {
    use super::*;
    use rayon::prelude::*;

    #[test]
    fn test_intcomma_parallel_mixed() {
        // Dataset: ints, floats, strings
        let numbers: Vec<(&str, PyObject)> = Python::with_gil(|py| {
            vec![
                ("int", 1i64.to_object(py)),
                ("int_neg", (-1234567i64).to_object(py)),
                ("float", 12345.678.to_object(py)),
                ("float_neg", (-9876.543).to_object(py)),
                ("string_num", "1234567890".to_object(py)),
                ("string_text", "hello world".to_object(py)),
            ]
        });

        // Parallel processing
        let results: Vec<(String, String)> = numbers
            .par_iter()
            .map(|(label, obj)| {
                Python::with_gil(|py| {
                    let formatted = intcomma(py, obj.clone_ref(py), Some(2)).unwrap();
                    (label.to_string(), formatted)
                })
            })
            .collect();

        // Spot check outputs
        for (label, output) in results {
            match label.as_str() {
                "int" => assert_eq!(output, "1"),
                "int_neg" => assert_eq!(output, "-1,234,567"),
                "float" => assert_eq!(output, "12,345.68"),
                "float_neg" => assert_eq!(output, "-9,876.54"),
                "string_num" => assert_eq!(output, "1,234,567,890"),
                "string_text" => assert_eq!(output, "hello world"),
                _ => panic!("Unexpected label"),
            }
        }
    }
}

// FIXME: Add benchmarks.
// FIXME: Add pre-commit.
