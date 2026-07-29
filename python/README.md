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
uuid_obj = fastuuidv7.uuid7()

same_text = fastuuidv7.format_uuid(raw)
same_text_from_obj = fastuuidv7.format_uuid(uuid_obj)
```

Convenience aliases:

```python
fastuuidv7.uuid7()      # fastuuidv7.UUID object
fastuuidv7.uuid7_str()  # str
fastuuidv7.uuid7_hex()  # str without dashes
fastuuidv7.uuid7_bytes()
```

## Benchmarks

```bash
cd python
python3 bench/bench.py
```

The benchmark measures repeated single calls, so the numbers include Python/native boundary overhead.

Current results on macOS arm64 with Python 3.14.4:

| Package | Callable | Time per call |
| --- | --- | ---: |
| `uuid-v7` | `uuid_v7.base.uuid7` | `2574.2 ns` |
| `uuid7` | `uuid_extensions.uuid7` | `1969.0 ns` |
| `uuid6` | `uuid6.uuid7` | `1946.2 ns` |
| `uuid` | `uuid.uuid7` | `1588.9 ns` |
| [`fastuuid7`](https://github.com/nekrasovp/uuidv7) | `fastuuid7.uuid7` | `126.3 ns` |
| [`uuid-utils`](https://github.com/aminalaee/uuid-utils) | `uuid_utils.uuid7` | `100.8 ns` |
| [`c-uuid-v7`](https://github.com/lava-sh/c_uuid_v7) | `c_uuid_v7.uuid7` | `40.7 ns` |
| `fastuuidv7` | `fastuuidv7.uuid7` | `22.2 ns` |

On linux x86 GH `c-uuid-v7` can still be about 5-10% faster than `fastuuidv7` - 40 ns vs 44ns. There are bench results that run for each fastuuidv7 release.

Notes:

- These are single-call throughput measurements, so they include Python/native boundary overhead.
- Install comparison packages such as `uuid-utils` before running; missing packages are skipped.

## Notes

- Python 3.14 includes `uuid.uuid7()` in the standard library.
- The main use case here is speed experiments and using a fast Rust-backed generator on older Python versions.
