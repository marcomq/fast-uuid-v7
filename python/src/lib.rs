use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyString, PyTuple};
#[cfg(not(Py_GIL_DISABLED))]
use pyo3::ffi;
use std::cell::Cell;
#[cfg(not(Py_GIL_DISABLED))]
use std::cell::RefCell;
#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

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

// Per-thread cache of a single UUID Python object.
//
// GIL builds only. The whole cache is compiled out under `Py_GIL_DISABLED`
// (free-threaded Python), where `uuid7_with_cache_id` allocates fresh instead
// (see below) — the refcount-based reuse trick relies on the GIL and has no
// safe, allocation-free equivalent without one.
//
// Uses `Py<UUID>` in a `RefCell` inside TLS. The `Py<UUID>` destructor
// decrefs the Python object when the TLS slot is reset or the thread exits.
// This is safe because thread exit normally happens while the interpreter
// (and the module) are still alive.
//
// The cache avoids the PyObject allocator on the hot path: when the cached
// UUID has refcount 1 (cache is sole owner), we mutate its inner `u128` via
// `Cell::set` through PyO3's `Bound::borrow()` and return it.
//
// Safety model:
// - The GIL serializes the refcount check + in-place mutation. On GIL builds
//   the GIL always exists; on free-threaded builds this code does not compile,
//   so the invariant cannot be violated.
// - Refcount check (==1) guarantees sole ownership before mutation.
// - `thread_local!` is per-OS-thread, not per-interpreter, so it does NOT by
//   itself isolate subinterpreters that share a thread. We rely instead on
//   PyO3's default of rejecting import into a subinterpreter with its own GIL;
//   without that a `Py<UUID>` cached under one interpreter could be handed to
//   another. If per-interpreter GIL support is ever enabled, this cache must
//   move to interpreter-keyed state cleaned up before finalization.
#[cfg(not(Py_GIL_DISABLED))]
thread_local! {
    static UUID_CACHE: RefCell<Option<Py<UUID>>> = const { RefCell::new(None) };
}

fn uuid7_with_cache(py: Python<'_>) -> Py<UUID> {
    uuid7_with_cache_id(py, fast_uuid_v7::gen_id())
}

#[inline(always)]
fn uuid7_with_cache_id(py: Python<'_>, id: u128) -> Py<UUID> {
    // Free-threaded (no-GIL) build: the reuse trick below relies on the GIL to
    // serialize the refcount check and in-place mutation, so we allocate a fresh
    // object each call. Generation stays fully parallel and lock-free (per-thread
    // generator state); only per-call allocation elision is given up.
    #[cfg(Py_GIL_DISABLED)]
    return Py::new(py, UUID::new(id)).expect("failed to allocate fastuuidv7.UUID");

    #[cfg(not(Py_GIL_DISABLED))]
    return UUID_CACHE.with(|cache| {
        let mut borrow = cache.borrow_mut();
        if let Some(cached) = borrow.as_ref() {
            // SAFETY: GIL held, pointer valid, no concurrent refcount changes.
            let reuse = unsafe {
                let ptr = cached.as_ptr() as *mut pyo3::ffi::PyObject;
                ffi::Py_REFCNT(ptr) == 1
            };

            if reuse {
                // Sole owner — mutate in-place via PyO3's borrow API.
                // SAFETY: refcount==1 confirms cache is the sole owner.
                // Cell::set provides interior mutability through &self
                // even though the pyclass is marked `frozen`.
                let bound = cached.bind(py);
                bound.borrow().id.set(id);
                return cached.clone_ref(py);
            }
        }

        // Cache empty or shared. Allocate fresh, cache it.
        let new = Py::new(py, UUID::new(id)).expect("failed to allocate fastuuidv7.UUID");
        *borrow = Some(new.clone_ref(py));
        new
    });
}

#[pyfunction]
fn reset_uuid_cache() {
    // No-op on free-threaded builds: there is no cache to clear.
    #[cfg(not(Py_GIL_DISABLED))]
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

/// Generates strictly increasing UUID v7 values from its own state.
///
/// Each instance guarantees that every generated id is greater than the
/// previous one, numerically and lexicographically. Intended for assigning ids
/// to rows read sequentially from a CSV / JSONL file, so the order survives
/// sorting by key, e.g. in S3.
///
/// The guarantee is per instance; it is not shared across instances or
/// processes. The state lives in the instance rather than in a thread-local, so
/// an instance may be handed from one thread to another and keeps its ordering
/// guarantee. It is not built for concurrent use from several threads at once —
/// give each thread its own instance.
#[pyclass(module = "fastuuidv7")]
struct SequentialGenerator {
    inner: fast_uuid_v7::SequentialGenerator,
}

#[pymethods]
impl SequentialGenerator {
    #[new]
    fn py_new() -> Self {
        Self {
            inner: fast_uuid_v7::SequentialGenerator::new(),
        }
    }

    fn next_id(&mut self) -> u128 {
        self.inner.next_id()
    }

    fn next_id_str<'py>(&mut self, py: Python<'py>) -> Bound<'py, PyString> {
        let uuid_str = self.inner.next_id_str();
        py_ascii_string_from_bytes(py, uuid_str.as_bytes())
    }

    fn next_id_bytes<'py>(&mut self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.next_id().to_be_bytes())
    }

    fn next_uuid(&mut self, py: Python<'_>) -> Py<UUID> {
        uuid7_with_cache_id(py, self.inner.next_id())
    }
}

// This module supports free-threaded (no-GIL) Python: the generator state is
// per-thread (`STATE` thread-local in the core crate), so ID generation is
// lock-free and fully parallel. The only GIL-dependent optimization is the
// `UUID`-object cache, which is compiled out on free-threaded builds (see
// `uuid7_with_cache_id`). Hence no `gil_used = true` opt-out is needed.
#[pymodule]
fn fastuuidv7(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<UUID>()?;
    m.add_class::<SequentialGenerator>()?;
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