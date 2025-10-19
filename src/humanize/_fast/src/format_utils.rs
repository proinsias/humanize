//! format_utils.rs
//!
//! Helper functions for formatting numbers/filesizes.

use regex::Regex;

/// Suffix tables
pub static SUFFIXES_DECIMAL: [&str; 10] = [
    " kB", " MB", " GB", " TB", " PB", " EB", " ZB", " YB", " RB", " QB",
];
pub static SUFFIXES_BINARY: [&str; 10] = [
    " KiB", " MiB", " GiB", " TiB", " PiB", " EiB", " ZiB", " YiB", " RiB", " QiB",
];
pub static SUFFIXES_GNU: &str = "KMGTPEZYRQ";

lazy_static::lazy_static! {
    static ref RE_FLOAT_FORMAT: Regex = Regex::new(r"%\.(\d+)f").unwrap();
}

/// Safely parse numeric-like strings to f64
pub fn parse_value(value: &str) -> Option<f64> {
    let cleaned = value.replace(',', "");
    cleaned.parse::<f64>().ok()
}

/// Normalize string representations of special float values to Python-style capitalization
pub fn normalize_special_values(s: &str) -> Option<String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "inf" | "+inf" => Some("+Inf".to_string()),
        "-inf" => Some("-Inf".to_string()),
        "nan" => Some("NaN".to_string()),
        _ => None,
    }
}

/// Format NaN/Inf values Python-style
pub fn format_not_finite(v: f64) -> String {
    if v.is_nan() {
        "NaN".to_string()
    } else if v.is_infinite() {
        if v.is_sign_positive() {
            "+Inf".to_string()
        } else {
            "-Inf".to_string()
        }
    } else {
        v.to_string()
    }
}

/// Insert commas into an integer string
pub fn add_commas(whole: &str) -> String {
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

/// Handle `%.Nf` style float formatting similar to Python’s printf
pub fn apply_printf_style(format_spec: &str, value: f64) -> String {
    if let Some(caps) = RE_FLOAT_FORMAT.captures(format_spec) {
        if let Some(m) = caps.get(1) {
            if let Ok(prec) = m.as_str().parse::<usize>() {
                return format!("{:.*}", prec, value);
            }
        }
    }
    value.to_string()
}
