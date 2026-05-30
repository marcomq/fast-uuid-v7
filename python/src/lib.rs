use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyString};

#[pyfunction]
fn gen_id() -> u128 {
    fast_uuid_v7::gen_id()
}

#[pyfunction]
fn gen_id_str(py: Python<'_>) -> Bound<'_, PyString> {
    let uuid_str = fast_uuid_v7::gen_id_str();
    PyString::new(py, uuid_str.as_ref())
}

#[pyfunction]
fn gen_id_bytes(py: Python<'_>) -> Bound<'_, PyBytes> {
    let bytes = fast_uuid_v7::gen_id().to_be_bytes();
    PyBytes::new(py, &bytes)
}

#[pyfunction]
fn format_uuid<'py>(py: Python<'py>, id: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyString>> {
    let id = id.extract::<u128>().map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(
            "id must be an integer in the range 0 <= id < 2**128",
        )
    })?;

    let formatted = fast_uuid_v7::format_uuid(id);
    Ok(PyString::new(py, formatted.as_ref()))
}

#[pymodule]
fn fastuuidv7(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gen_id, m)?)?;
    m.add_function(wrap_pyfunction!(gen_id_str, m)?)?;
    m.add_function(wrap_pyfunction!(gen_id_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(format_uuid, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    m.add("uuid7", m.getattr("gen_id_str")?)?;
    m.add("uuid7_bytes", m.getattr("gen_id_bytes")?)?;

    Ok(())
}
