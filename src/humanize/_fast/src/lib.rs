use pyo3::prelude::*;

// Declare your other modules
mod filesize;
mod format_utils;
mod number;

use filesize::naturalsize;
use number::*;

/// PyO3 module entrypoint — defines what Python sees as `_fast`
#[pymodule]
fn _fast(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add PyO3 functions from each submodule
    m.add_function(wrap_pyfunction_bound!(intcomma, m)?)?;
    m.add_function(wrap_pyfunction_bound!(intword, m)?)?;
    m.add_function(wrap_pyfunction_bound!(naturalsize, m)?)?;
    Ok(())
}
