use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyString, PyTuple};
use std::cell::{Cell, RefCell};

thread_local! {
    static UUID_CACHE: RefCell<Option<Py<UUID>>> = const { RefCell::new(None) };
}

#[pyclass(
    module = "fastuuidv7",
    frozen,
    freelist = 1024,
    unsendable,
    skip_from_py_object
)]
struct UUID {
    id: Cell<u128>,
}

fn parse_uuid_text(text: &str) -> PyResult<u128> {
    let mut value = 0u128;
    let mut digits = 0usize;

    for byte in text.bytes() {
        if byte == b'-' {
            continue;
        }

        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => {
                return Err(PyValueError::new_err(
                    "badly formed hexadecimal UUID string",
                ));
            }
        };

        if digits == 32 {
            return Err(PyValueError::new_err(
                "badly formed hexadecimal UUID string",
            ));
        }

        value = (value << 4) | u128::from(digit);
        digits += 1;
    }

    if digits != 32 {
        return Err(PyValueError::new_err(
            "badly formed hexadecimal UUID string",
        ));
    }

    Ok(value)
}

fn parse_uuid_value(value: &Bound<'_, PyAny>) -> PyResult<u128> {
    if let Ok(uuid) = value.extract::<PyRef<'_, UUID>>() {
        return Ok(uuid.id.get());
    }

    if let Ok(id) = value.extract::<u128>() {
        return Ok(id);
    }

    let text = value
        .extract::<&str>()
        .map_err(|_| PyValueError::new_err("UUID value must be an int, str, or fastuuidv7.UUID"))?;
    parse_uuid_text(text)
}

fn uuid_hex_bytes(id: u128) -> [u8; 32] {
    *fast_uuid_v7::format_uuid_hex(id).as_bytes()
}

impl UUID {
    #[inline]
    fn new(id: u128) -> Self {
        Self { id: Cell::new(id) }
    }
}

#[pymethods]
impl UUID {
    #[new]
    fn py_new(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(parse_uuid_value(value)?))
    }

    fn __str__<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        let formatted = fast_uuid_v7::format_uuid(self.id.get());
        PyString::new(py, formatted.as_ref())
    }

    fn __repr__(&self) -> String {
        format!("UUID('{}')", fast_uuid_v7::format_uuid(self.id.get()))
    }

    fn __int__(&self) -> u128 {
        self.id.get()
    }

    fn __index__(&self) -> u128 {
        self.id.get()
    }

    fn __hash__(&self) -> isize {
        let id = self.id.get();
        let hi = (id >> 64) as u64;
        let lo = id as u64;
        let hash = (hi ^ (hi >> 32) ^ lo ^ (lo >> 32)) as isize;
        if hash == -1 { -2 } else { hash }
    }

    fn __richcmp__(&self, other: PyRef<'_, UUID>, op: CompareOp) -> bool {
        let id = self.id.get();
        let other_id = other.id.get();
        match op {
            CompareOp::Lt => id < other_id,
            CompareOp::Le => id <= other_id,
            CompareOp::Eq => id == other_id,
            CompareOp::Ne => id != other_id,
            CompareOp::Gt => id > other_id,
            CompareOp::Ge => id >= other_id,
        }
    }

    #[getter]
    fn int(&self) -> u128 {
        self.id.get()
    }

    #[getter]
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.id.get().to_be_bytes())
    }

    #[getter]
    fn hex<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        let hex = uuid_hex_bytes(self.id.get());
        let text = std::str::from_utf8(&hex)
            .map_err(|_| PyValueError::new_err("UUID hex contained invalid UTF-8"))?;
        Ok(PyString::new(py, text))
    }

    #[getter]
    fn urn(&self) -> String {
        format!("urn:uuid:{}", fast_uuid_v7::format_uuid(self.id.get()))
    }

    #[getter]
    fn time(&self) -> u64 {
        (self.id.get() >> 80) as u64
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        (self.id.get() >> 80) as u64
    }

    #[getter]
    fn version(&self) -> u8 {
        ((self.id.get() >> 76) & 0x0f) as u8
    }

    #[getter]
    fn variant(&self) -> &'static str {
        "specified in RFC 4122"
    }

    #[getter]
    fn fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        let id = self.id.get();
        PyTuple::new(
            py,
            [
                (id >> 96) as u64,
                ((id >> 80) & 0xffff) as u64,
                ((id >> 64) & 0xffff) as u64,
                ((id >> 56) & 0xff) as u64,
                ((id >> 48) & 0xff) as u64,
                (id & 0xffff_ffff_ffff) as u64,
            ],
        )
    }
}

#[pyfunction]
fn gen_id() -> u128 {
    fast_uuid_v7::gen_id()
}

#[pyfunction]
fn gen_id_with_sub_ms_4() -> u128 {
    fast_uuid_v7::gen_id_with_sub_ms_4()
}

#[pyfunction]
fn gen_id_with_sub_ms_8() -> u128 {
    fast_uuid_v7::gen_id_with_sub_ms_8()
}

#[pyfunction]
fn gen_id_with_sub_ms_12() -> u128 {
    fast_uuid_v7::gen_id_with_sub_ms_12()
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
fn uuid7(py: Python<'_>) -> Py<UUID> {
    let id = fast_uuid_v7::gen_id();

    UUID_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();

        if let Some(uuid) = cache.as_ref() {
            let refcnt = unsafe { pyo3::ffi::Py_REFCNT(uuid.as_ptr()) };
            if refcnt == 1 {
                uuid.bind(py).borrow().id.set(id);
                return uuid.clone_ref(py);
            }
        }

        let uuid = Py::new(py, UUID::new(id)).expect("failed to allocate fastuuidv7.UUID");
        *cache = Some(uuid.clone_ref(py));
        uuid
    })
}

#[pyfunction]
fn uuid7_str(py: Python<'_>) -> Bound<'_, PyString> {
    gen_id_str(py)
}

#[pyfunction]
fn uuid7_hex<'py>(py: Python<'py>) -> Bound<'py, PyString> {
    let uuid_hex = fast_uuid_v7::format_uuid_hex(fast_uuid_v7::gen_id());
    PyString::new(py, uuid_hex.as_ref())
}

#[pyfunction]
fn format_uuid<'py>(py: Python<'py>, id: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyString>> {
    let id = parse_uuid_value(id)?;

    let formatted = fast_uuid_v7::format_uuid(id);
    Ok(PyString::new(py, formatted.as_ref()))
}

#[pymodule]
fn fastuuidv7(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<UUID>()?;
    m.add_function(wrap_pyfunction!(gen_id, m)?)?;
    m.add_function(wrap_pyfunction!(gen_id_with_sub_ms_4, m)?)?;
    m.add_function(wrap_pyfunction!(gen_id_with_sub_ms_8, m)?)?;
    m.add_function(wrap_pyfunction!(gen_id_with_sub_ms_12, m)?)?;
    m.add_function(wrap_pyfunction!(gen_id_str, m)?)?;
    m.add_function(wrap_pyfunction!(gen_id_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(uuid7, m)?)?;
    m.add_function(wrap_pyfunction!(uuid7_str, m)?)?;
    m.add_function(wrap_pyfunction!(uuid7_hex, m)?)?;
    m.add_function(wrap_pyfunction!(format_uuid, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    m.add("uuid7_bytes", m.getattr("gen_id_bytes")?)?;

    Ok(())
}
