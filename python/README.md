# fastuuidv7

Python bindings for the Rust [`fast-uuid-v7`](https://github.com/marcomq/fast-uuid-v7) generator.

[![Python >=3.8](https://img.shields.io/badge/python-%3E%3D3.8-3776AB?logo=python&logoColor=white)](https://pypi.org/project/fastuuidv7/)
![Linux x86_64/aarch64/armv7](https://img.shields.io/badge/linux-x86__64%20%7C%20aarch64%20%7C%20armv7-FCC624?logo=linux&logoColor=black)
![macOS x86_64/aarch64](https://img.shields.io/badge/macos-x86__64%20%7C%20aarch64-000000?logo=apple&logoColor=white)
![Windows x86_64](https://img.shields.io/badge/windows-x86__64-0078D6?logo=windows&logoColor=white)

This package is intentionally small. It exposes the fast Rust UUIDv7 generator to Python with a module name of `fastuuidv7` and keeps the API close to the Rust crate.

## Supported platforms

Prebuilt wheels are currently published for:

- Linux: `x86_64`, `aarch64`, `armv7`
- macOS: `x86_64`, `aarch64`
- Windows: `x86_64`

Windows ARM64 is not part of the current wheel build matrix yet, but source builds remain possible with a local Rust toolchain and Python development environment.

## Install locally

```bash
cd python
maturin develop
```

## Usage

```python
import fastuuidv7

raw = fastuuidv7.gen_id()
raw_sub_ms_4 = fastuuidv7.gen_id_with_sub_ms_4()
raw_sub_ms_8 = fastuuidv7.gen_id_with_sub_ms_8()
raw_sub_ms_12 = fastuuidv7.gen_id_with_sub_ms_12()
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

Current results on linux amd64_x86 git runner:

| Package | Callable | Time per call |
| --- | --- | ---: |
| `uuid6` | `uuid6.uuid7` | `2890.4 ns` |
| `uuid7` | `uuid_extensions.uuid7` | `2767.7 ns` |
| `fastuuid7` | `uuidv7.uuid7` | `313.5 ns` |
| `fastuuidv7 ` | `fastuuidv7.uuid7` | `60.6 ns` |

Notes:

- These are single-call throughput measurements, so they include Python/native boundary overhead.

## Notes

- Python 3.14 includes `uuid.uuid7()` in the standard library.
- The main use case here is speed experiments and using a fast Rust-backed generator on older Python versions.
