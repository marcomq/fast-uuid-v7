use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyModule};

#[pyfunction]
fn gen_id() -> u128 {
    fast_uuid_v7::gen_id()
}

#[pyfunction]
fn gen_id_str() -> String {
    fast_uuid_v7::gen_id_str().to_string()
}

#[pyfunction]
fn gen_id_bytes(py: Python<'_>) -> Bound<'_, PyBytes> {
    let bytes = fast_uuid_v7::gen_id().to_be_bytes();
    PyBytes::new(py, &bytes)
}

#[pyfunction]
fn format_uuid(id: &Bound<'_, PyAny>) -> PyResult<String> {
    let id = id.extract::<u128>().map_err(|_| {
        PyValueError::new_err("id must be an integer in the range 0 <= id < 2**128")
    })?;

    Ok(fast_uuid_v7::format_uuid(id).to_string())
}

#[pymodule]
fn fastuuidv7(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gen_id, m)?)?;
    m.add_function(wrap_pyfunction!(gen_id_str, m)?)?;
    m.add_function(wrap_pyfunction!(gen_id_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(format_uuid, m)?)?;

    m.add("uuid7", m.getattr("gen_id_str")?)?;
    m.add("uuid7_bytes", m.getattr("gen_id_bytes")?)?;

    Ok(())
}
