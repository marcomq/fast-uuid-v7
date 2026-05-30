# fastuuidv7

Python bindings for the Rust [`fast-uuid-v7`](..) generator.

This package is intentionally small. It exposes the fast Rust UUIDv7 generator to Python with a module name of `fastuuidv7` and keeps the API close to the Rust crate.

## Install locally

```bash
cd python
maturin develop
```

## Usage

```python
import fastuuidv7

raw = fastuuidv7.gen_id()
text = fastuuidv7.gen_id_str()
data = fastuuidv7.gen_id_bytes()

same_text = fastuuidv7.format_uuid(raw)
```

Convenience aliases:

```python
fastuuidv7.uuid7()
fastuuidv7.uuid7_bytes()
```

## Benchmarks

```bash
cd python
python3 bench/bench.py
```

The benchmark measures repeated single calls, so the numbers include Python/native boundary overhead.

Current results on linux git runner:

| Package | Callable | Time per call |
| --- | --- | ---: |
| `uuid6` | `uuid6.uuid7` | `2890.4 ns/call` |
| `uuid7` | `uuid_extensions.uuid7` | `2767.7 ns` |
| `fastuuid7` | `uuidv7.uuid7` | `313.5 ns` |
| `fastuuidv7 ` | `fastuuidv7.uuid7` | `60.6 ns` |

Notes:

- These are single-call throughput measurements, so they include Python/native boundary overhead.

## Notes

- Python 3.14 includes `uuid.uuid7()` in the standard library.
- The main use case here is speed experiments and using a fast Rust-backed generator on older Python versions.
