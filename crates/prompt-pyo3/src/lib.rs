use pyo3::prelude::*;

/// Python bindings for the prompt-core key input parser.
/// 
/// This module provides Python access to the Rust-based key input parser
/// that can handle raw terminal input and convert byte sequences to 
/// structured key events.
#[pymodule]
fn prompt(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}