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

Current local results on this machine:

| Package | Callable | Time per call |
| --- | --- | ---: |
| `fastuuidv7` | `fastuuidv7.uuid7` | `246.6 ns` |
| `fastuuid7` | `uuidv7.uuid7` | `275.8 ns` |
| `uuid` stdlib | `uuid.uuid7` | `1608.9 ns` |
| `uuid6` | `uuid6.uuid7` | `1977.0 ns` |
| `uuid7` | `uuid_extensions.uuid7` | `1988.9 ns` |
| `uuid-v7` | `uuid_v7.base.uuid7` | `2651.9 ns` |

Notes:

- These are single-call throughput measurements, so they include Python/native boundary overhead.
- The `fastuuid7` result above came from a local macOS compatibility patch to its published source package, which currently links Linux `librt` unconditionally.

## Notes

- Python 3.14 includes `uuid.uuid7()` in the standard library.
- The main use case here is speed experiments and using a fast Rust-backed generator on older Python versions.
