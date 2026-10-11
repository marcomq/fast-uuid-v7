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

### Strictly increasing ids

`SequentialGenerator` guarantees that every id is greater than the previous one
from the same instance, numerically and lexicographically. Use one instance per
input file when ingesting CSV / JSONL rows, so the input order survives sorting
by key.

```python
gen = fastuuidv7.SequentialGenerator()

for row in reader:
    row["id"] = gen.next_id_str()   # also: next_id(), next_id_bytes(), next_uuid()
```

The guarantee is per instance — it is not shared across instances or processes.
An instance may be handed from one thread to another and keeps its ordering
guarantee, but it is not built for concurrent use from several threads at once;
give each thread its own instance.

## Benchmarks

```bash
cd python
python3 bench/bench.py
# optionally append the results as a Markdown table:
python3 bench/bench.py --markdown results.md
```

The benchmark measures repeated single calls, so the numbers include Python/native boundary overhead. It reports the fastest of 3 runs of 500,000 calls each. Install comparison packages such as `uuid-utils` before running; missing packages are skipped.

Results from the [Python Benchmark workflow](https://github.com/marcomq/fast-uuid-v7/actions/workflows/benchmark-python.yml) on GitHub-hosted runners with Python 3.11, `fastuuidv7` 0.1.10 built from commit `7601a77` ([run of 2026-10-11](https://github.com/marcomq/fast-uuid-v7/actions/runs/38155377723)), time per call:

| Package | Callable | Linux x86_64 | Windows x86_64 | macOS arm64 |
| --- | --- | ---: | ---: | ---: |
| `uuid-v7` 1.0.0 | `uuid_v7.base.uuid7` | `2287.1 ns` | `3388.8 ns` | `2337.4 ns` |
| `uuid7` 0.1.0 | `uuid_extensions.uuid7` | `1648.4 ns` | `2065.3 ns` | `1728.1 ns` |
| `uuid6` 2025.0.1 | `uuid6.uuid7` | `1665.1 ns` | `2262.7 ns` | `1929.3 ns` |
| [`fastuuid7`](https://github.com/nekrasovp/uuidv7) 0.5.0 | `fastuuid7.uuid7` | `270.9 ns` | `144.2 ns` | `123.8 ns` |
| [`uuid-utils`](https://github.com/aminalaee/uuid-utils) 1.0.0 | `uuid_utils.uuid7` | `88.4 ns` | `157.0 ns` | `122.5 ns` |
| [`c-uuid-v7`](https://github.com/lava-sh/c_uuid_v7) 0.0.11 | `str(c_uuid_v7.uuid7())` | `45.1 ns` | `91.2 ns` | `128.3 ns` |
| [`c-uuid-v7`](https://github.com/lava-sh/c_uuid_v7) 0.0.11 | `c_uuid_v7.uuid7` | `27.5 ns` | `53.2 ns` | `61.0 ns` |
| `fastuuidv7` | `fastuuidv7.uuid7_str` | `25.4 ns` | `60.4 ns` | `31.3 ns` |
| `fastuuidv7` | `fastuuidv7.uuid7` | `19.0 ns` | `42.1 ns` | `20.0 ns` |

`c-uuid-v7` has no function that returns a string directly, so `str(c_uuid_v7.uuid7())` is its counterpart to `fastuuidv7.uuid7_str`.

How to read these numbers:

- Every run of the workflow publishes the full tables, including the runner's CPU, on the summary page of that run. It runs for each change to the package on `main` and for pull requests.
- Runner hardware differs from run to run (this run: AMD EPYC 9V45 on Linux, an AMD family 25 CPU on Windows), and absolute times have moved by 20-40% between two runs of the same code. Compare packages within one run, not absolute times across runs.
- In this run `fastuuidv7.uuid7` was about 31% faster than `c_uuid_v7.uuid7` on Linux and 21% faster on Windows. Treat the size of that lead with care: two runs of the same 0.1.9 wheel on the same day put it 8% ahead on Linux in one and 19% behind in the other.
- The standard library's `uuid.uuid7` needs Python 3.14 and is therefore missing from the CI table. A local run on macOS arm64 with Python 3.14.4 measured `1588.9 ns` per call.

## Notes

- Python 3.14 includes `uuid.uuid7()` in the standard library.
- The main use case here is speed experiments and using a fast Rust-backed generator on older Python versions.
