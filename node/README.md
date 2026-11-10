# fast-uuid-v7 (Node.js)

Node.js bindings for the Rust [`fast-uuid-v7`](https://github.com/marcomq/fast-uuid-v7) generator, built with [napi-rs](https://napi.rs). Works in Node.js (>= 18), Deno and Bun.

```bash
npm install fast-uuid-v7
```

Prebuilt binaries are published for:

- Linux (glibc 2.31+): `x64`, `arm64`
- macOS: `x64`, `arm64`
- Windows: `x64`

## Usage

```js
const { uuid7, genId, genIdBuffer, fillIds, SequentialGenerator } = require("fast-uuid-v7");
// or: import { uuid7 } from "fast-uuid-v7";

uuid7();          // "019a2b3c-4d5e-7f60-8123-456789abcdef"
genId();          // 2131...n (BigInt)
genIdBuffer();    // <Buffer 01 9a ...> (16 bytes, big-endian)

// Batch: fill a buffer with as many ids as fit, returns the count
const batch = Buffer.alloc(16 * 1000);
fillIds(batch);
```

| Function                                  | Returns  | Notes                                                    |
|-------------------------------------------|----------|----------------------------------------------------------|
| `genIdStr()` / `uuid7()`                  | `string` | Canonical `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`         |
| `genIdHex()` / `uuid7Hex()`               | `string` | 32 hex chars without dashes                              |
| `genIdBuffer()` / `uuid7Buffer()`         | `Buffer` | 16 bytes; a view into a shared pool, like `Buffer.allocUnsafe` |
| `genId()`                                 | `bigint` | 74 random bits                                           |
| `fillIds(target)`                         | `number` | Fills `target` with 16-byte ids, returns the count       |
| `genIdWithCount()` / `genIdWithCountStr()`| `bigint` / `string` | Monotonic per thread (18-bit counter + 56 random bits) |
| `genIdWithSubMs4/8/12()`                  | `bigint` | Sub-millisecond fraction in `rand_a`                     |
| `formatUuid(id)`                          | `string` | From a `bigint` or 16-byte buffer                        |

See the [main README](../README.md#randomness-vs-monotonicity) for the randomness and ordering trade-offs of each variant.

### Strictly increasing ids

`SequentialGenerator` guarantees that every id is greater than the previous one from the same instance, numerically, lexicographically and byte-wise. Use one instance per input file when ingesting CSV / JSONL rows, so the input order survives sorting by key.

```js
const gen = new SequentialGenerator();

for (const row of rows) {
  row.id = gen.nextIdStr(); // also: nextId(), nextIdBuffer(), fillIds(buffer)
}
```

The generator is also an infinite iterator of `bigint` ids. The guarantee is per instance, not across instances, workers or processes.

### Worker threads

Each worker thread has its own generator state, so the functions above need no locking and do not contend across workers.

## Benchmarks

```bash
npm run bench
```

Measured on an Apple M1, nanoseconds per id:

| Function                         | Node 24 | Deno 2.9 | Bun 1.4 |
|----------------------------------|--------:|---------:|--------:|
| `genIdStr`                       |      45 |       48 |      89 |
| `genIdBuffer`                    |      66 |       56 |      49 |
| `genId` (bigint)                 |      71 |       72 |      59 |
| `fillIds` (batch of 1024)        |     7.4 |      7.6 |     7.5 |
| `SequentialGenerator.nextIdStr`  |      75 |       71 |     122 |
| `uuid` v7() (npm)                |    1758 |      679 |     291 |
| `uuidv7` (npm)                   |     581 |      600 |     273 |
| `crypto.randomUUID()` (v4)       |     217 |      109 |      69 |
| `Bun.randomUUIDv7()`             |         |          |      82 |

Bun ships a fast built-in `Bun.randomUUIDv7()`, so on Bun this package mostly helps with `SequentialGenerator`, batch generation and buffers.

## Deno

```js
import { uuid7 } from "npm:fast-uuid-v7";
```

Native addons need `--allow-ffi` (plus `--allow-read` and `--allow-env` for the loader).

## Build from source

Requires a Rust toolchain (>= 1.88).

```bash
cd node
npm install
npm run build
npm test
```

## Limitations

Not cryptographically secure. See the [main README](../README.md#limitations).
