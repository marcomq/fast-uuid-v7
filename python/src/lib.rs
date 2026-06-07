use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyString, PyTuple};
use pyo3::ffi;
use std::cell::{Cell, RefCell};

#[cfg_attr(
    any(Py_3_14, all(Py_3_10, not(Py_LIMITED_API))),
    pyclass(
        module = "fastuuidv7",
        frozen,
        freelist = 1024,
        immutable_type,
        unsendable,
        skip_from_py_object
    )
)]
#[cfg_attr(
    not(any(Py_3_14, all(Py_3_10, not(Py_LIMITED_API)))),
    pyclass(
        module = "fastuuidv7",
        frozen,
        freelist = 1024,
        unsendable,
        skip_from_py_object
    )
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

#[inline]
fn py_ascii_string_from_bytes<'py>(py: Python<'py>, bytes: &[u8]) -> Bound<'py, PyString> {
    debug_assert!(bytes.is_ascii());

    #[cfg(not(any(Py_LIMITED_API, PyPy, GraalPy)))]
    {
        unsafe {
            let ptr = pyo3::ffi::PyUnicode_New(bytes.len() as pyo3::ffi::Py_ssize_t, 127);
            if ptr.is_null() {
                // SAFETY: UUID formatting emits only ASCII hex digits and dashes.
                let text = std::str::from_utf8_unchecked(bytes);
                return PyString::new(py, text);
            }
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                pyo3::ffi::PyUnicode_1BYTE_DATA(ptr),
                bytes.len(),
            );
            Bound::from_owned_ptr(py, ptr).cast_into_unchecked()
        }
    }

    #[cfg(any(Py_LIMITED_API, PyPy, GraalPy))]
    {
        // SAFETY: UUID formatting emits only ASCII hex digits and dashes.
        let text = unsafe { std::str::from_utf8_unchecked(bytes) };
        PyString::new(py, text)
    }
}

impl UUID {
    #[inline]
    fn new(id: u128) -> Self {
        Self { id: Cell::new(id) }
    }
}

thread_local! {
    static UUID_CACHE: RefCell<Option<Py<UUID>>> = const { RefCell::new(None) };
}

fn uuid7_with_cache(py: Python<'_>) -> Py<UUID> {
    uuid7_with_cache_id(py, fast_uuid_v7::gen_id())
}

fn uuid7_with_cache_id(py: Python<'_>, id: u128) -> Py<UUID> {
    UUID_CACHE.with(|cache| {
        let mut borrow = cache.borrow_mut();
        if let Some(cached) = borrow.as_ref() {
            // Check if the cache is the sole owner (refcount == 1).
            // SAFETY: We hold the GIL, so no concurrent refcount changes.
            // The pointer is valid because we own a `Py<UUID>` in the cache.
            let reuse = unsafe {
                let ptr = cached.as_ptr() as *mut pyo3::ffi::PyObject;
                let refcnt = ffi::Py_REFCNT(ptr);
                #[cfg(not(any(Py_LIMITED_API, PyPy, GraalPy)))]
                {
                    // refcnt == 1: cache is sole owner, safe to mutate.
                    // Immortal objects (refcnt == IMMORTAL_SENTINEL) must NOT
                    // be mutated in-place because we cannot distinguish "cache
                    // is sole owner" from "many code paths share this object".
                    refcnt == 1
                }
                #[cfg(any(Py_LIMITED_API, PyPy, GraalPy))]
                {
                    refcnt == 1
                }
            };

            if reuse {
                // We own the only reference — mutate in-place.
                // SAFETY: The pyclass is `frozen`, but `Cell::set` provides
                // interior mutability via an immutable reference. We are
                // allowed to mutate the Rust value through Cell.
                let bound = cached.bind(py);
                let uuid_ref = bound.borrow();
                uuid_ref.id.set(id);
                drop(uuid_ref);
                return cached.clone_ref(py);
            }
        }

        // Cache is empty, or the cached object is shared elsewhere.
        // Allocate a fresh UUID and update the cache for next time.
        let new = Py::new(py, UUID::new(id)).expect("failed to allocate fastuuidv7.UUID");
        *borrow = Some(new.clone_ref(py));
        new
    })
}

#[pyfunction]
fn reset_uuid_cache() {
    UUID_CACHE.with(|cache| {
        *cache.borrow_mut() = None;
    });
}

#[pymethods]
impl UUID {
    #[new]
    fn py_new(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self::new(parse_uuid_value(value)?))
    }

    fn __str__<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        let formatted = fast_uuid_v7::format_uuid(self.id.get());
        py_ascii_string_from_bytes(py, formatted.as_bytes())
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
        Ok(py_ascii_string_from_bytes(py, &hex))
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
        let clock_seq_hi = ((self.id.get() >> 56) & 0xff) as u8;

        if clock_seq_hi & 0x80 == 0 {
            "reserved for NCS compatibility"
        } else if clock_seq_hi & 0xc0 == 0x80 {
            "specified in RFC 4122"
        } else if clock_seq_hi & 0xe0 == 0xc0 {
            "reserved for Microsoft compatibility"
        } else {
            "reserved for future definition"
        }
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
    py_ascii_string_from_bytes(py, uuid_str.as_bytes())
}

#[pyfunction]
fn gen_id_bytes(py: Python<'_>) -> Bound<'_, PyBytes> {
    let bytes = fast_uuid_v7::gen_id().to_be_bytes();
    PyBytes::new(py, &bytes)
}

#[pyfunction]
fn uuid7(py: Python<'_>) -> Py<UUID> {
    uuid7_with_cache(py)
}

#[pyfunction]
fn uuid7_str(py: Python<'_>) -> Bound<'_, PyString> {
    gen_id_str(py)
}

#[pyfunction]
fn uuid7_hex<'py>(py: Python<'py>) -> Bound<'py, PyString> {
    let uuid_hex = fast_uuid_v7::format_uuid_hex(fast_uuid_v7::gen_id());
    py_ascii_string_from_bytes(py, uuid_hex.as_bytes())
}

#[pyfunction]
fn format_uuid<'py>(py: Python<'py>, id: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyString>> {
    let id = parse_uuid_value(id)?;

    let formatted = fast_uuid_v7::format_uuid(id);
    Ok(py_ascii_string_from_bytes(py, formatted.as_bytes()))
}

#[pyfunction]
fn uuid7_with_count(py: Python<'_>) -> Py<UUID> {
    uuid7_with_cache_id(py, fast_uuid_v7::gen_id_with_count())
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
    m.add_function(wrap_pyfunction!(reset_uuid_cache, m)?)?;
    m.add_function(wrap_pyfunction!(uuid7_with_count, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    m.add("uuid7_bytes", m.getattr("gen_id_bytes")?)?;

    Ok(())
}