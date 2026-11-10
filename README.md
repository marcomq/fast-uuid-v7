# fast-uuid-v7

A high-performance Rust library for generating UUID v7 compatible identifiers.

This implementation focuses on speed. The `gen_id*` functions use thread-local state and an OS-seeded `SmallRng`, so they generate IDs without locks, making them suitable for high-throughput applications.

## Installation

```bash
cargo add fast-uuid-v7
```

## Features

*   **UUID v7**: Time-ordered, 128-bit unique identifiers.
*   **Fast**: Minimal overhead using thread-local state, cached timestamps and SIMD formatting.
*   **Flexible**: Choose between maximum randomness (`gen_id`), optional sub-millisecond sort locality (`gen_id_with_sub_ms_4`, `gen_id_with_sub_ms_8`, `gen_id_with_sub_ms_12`), per-thread monotonicity (`gen_id_with_count`), or strict per-instance ordering (`SequentialGenerator`).
*   **Owned generators**: `SequentialGenerator` and `RandomGenerator` keep their own state instead of using thread-local storage.
*   **Fork-aware**: `reseed()` re-seeds the generator in a forked child process.

## Why?

I was testing the performance of network streams and used the original `uuid` v7 to generate messages
with unique IDs. I wondered why there was a maximum of 400,000 messages per 
second, even though the rest of the code looked like it could handle millions.
I figured out that 400,000 is mostly the limit of the standard `uuid` crate when
using v7 without any additional feature flags. This happened due to the strong
random number generator used in `uuid`. While a strong RNG is correct and avoids potential issues
in security or crypto contexts, it is otherwise just very slow.
I found out about the `fast-rng` feature flag of `uuid` after creating this crate. But even with `fast-rng` feature flag, `fast-uuid-v7` is much faster than `uuid`.

## Comparison to `uuid` crate

The numbers below were measured locally on an Apple M1 and vary by hardware; see [Benchmarking](#benchmarking) to measure them on yours.

Compared to the standard `uuid` crate with its default secure RNG (`Uuid::now_v7()`, ~1.4µs per ID):
*   **`gen_id` is around ~180x faster** (7.9ns vs 1.43µs).
*   **`gen_id_str` is around ~135x faster** than `Uuid::now_v7().to_string()` (11ns vs 1.5µs).
*   With the `fast-rng` feature of the `uuid` crate the gap is smaller, but `fast-uuid-v7` was still about 8x faster for `u128` and 11x faster for strings in earlier measurements.

## Randomness vs Monotonicity

### `gen_id` (74 bits randomness)
The default `gen_id` (and `gen_id_u128`) uses all available 74 bits for randomness. This matches the standard UUID v7 randomness layout.
*   **Pros**: Extremely low collision risk across distributed systems.
*   **Cons**: IDs generated within the same millisecond on the same thread are not guaranteed to be monotonic (they will be random).

### `gen_id_with_count` (56 bits randomness + 18-bit counter)
The `gen_id_with_count` function uses an 18-bit counter and 56 bits of randomness. On each new millisecond tick, the counter is RFC-style seeded before continuing monotonically on that thread.
*   **Pros**: Guarantees monotonicity per thread (up to ~262k IDs/ms).
*   **Cons**: Reduced randomness (56 bits) increases collision risk in massive distributed systems (approx. 50% chance after 4.5 billion IDs/ms globally).

### `SequentialGenerator` (56 bits randomness + seeded 18-bit counter)
`SequentialGenerator` owns its state instead of using the thread-local fast path, and guarantees that every id is strictly greater than the previous one from the same instance — numerically and lexicographically. The counter is randomly seeded in its low 12 bits on each new millisecond as RFC 9562 §6.2 prescribes, the timestamp never moves backwards, and counter exhaustion advances the timestamp by 1ms. Useful for assigning ids to rows read sequentially from a CSV / JSONL file, so the input order survives sorting by key, e.g. in S3. The generator is also an infinite `Iterator`, so ids can be zipped straight onto rows.
*   **Pros**: Strict ordering guarantee, no effect on the `gen_id_*` functions.
*   **Cons**: Guarantee is per instance only — not across instances, threads or processes. Needs `&mut self`; share it across threads only behind your own lock. Same 56-bit randomness trade-off as `gen_id_with_count`. At least 258,049 ids per millisecond before the timestamp is pushed ahead of the wall clock; a tight generation loop can outrun that, though it catches up as soon as generation pauses.

### `gen_id_with_sub_ms_4`, `gen_id_with_sub_ms_8`, `gen_id_with_sub_ms_12`
These functions keep the standard 48-bit millisecond timestamp, then place a scaled sub-millisecond fraction into the high bits of `rand_a` as described by RFC 9562. The remaining bits of `rand_a` stay random, and `rand_b` stays fully random. The millisecond timestamp comes from wall-clock time, but on supported counter backends the sub-millisecond fraction is often estimated between wall-clock refreshes instead of being freshly measured on every call.
*   **Pros**: Improves sort locality for IDs created within the same millisecond without adding counters or shared state.
*   **Cons**: This is not true nanosecond ordering, and it does not provide distributed monotonicity.

### `RandomGenerator` (74 bits randomness)
Owned-state counterpart of `gen_id`: same layout, but generated from the instance instead of thread-local state. Useful when the caller already serializes access (e.g. behind a lock) and wants to skip the thread-local lookup.

## Bit Layout

The 128-bit ID is fully compatible with UUID v7. It is composed of:

*   **48 bits**: Unix timestamp in milliseconds.
*   **4 bits**: Version (7).
*   **12 bits**: Random Data (or Counter High 12 bits with `gen_id_with_count` / `SequentialGenerator`, or 4/8/12 bits sub-millisecond fraction + remaining random bits with `gen_id_with_sub_ms_*`).
*   **2 bits**: Variant (10xx).
*   **62 bits**: Random Data (or Counter Low 6 bits + 56 bits random with `gen_id_with_count` / `SequentialGenerator`).

**Total Randomness:**
*   `gen_id` / `RandomGenerator`: **74 bits**
*   `gen_id_with_sub_ms_4`: **70 bits**
*   `gen_id_with_sub_ms_8`: **66 bits**
*   `gen_id_with_sub_ms_12`: **62 bits**
*   `gen_id_with_count`: **56 bits**
*   `SequentialGenerator`: **56 bits**

## Usage

```rust
use fast_uuid_v7::{
    format_uuid_hex, gen_id, gen_id_str, gen_id_string, gen_id_with_count,
    gen_id_with_count_str, gen_id_with_sub_ms_8, RandomGenerator, SequentialGenerator,
};

fn main() {
    // Get ID as u128 (74 bits random)
    let id = gen_id();
    println!("Generated ID: {:032x}", id);

    // Get per-thread monotonic ID (56 bits random + 18-bit counter)
    let ordered_id = gen_id_with_count();
    println!("Ordered ID: {:032x}", ordered_id);
    println!("Ordered ID string: {}", gen_id_with_count_str());

    // Strictly increasing IDs from one instance, e.g. per input file
    let mut sequential = SequentialGenerator::new();
    println!("Row 1: {}", sequential.next_id_str());
    println!("Row 2: {}", sequential.next_id_str());
    // SequentialGenerator is also an infinite Iterator<Item = u128>
    let ids: Vec<u128> = sequential.by_ref().take(3).collect();
    println!("Batch: {:032x?}", ids);

    // Owned-state variant of gen_id, e.g. for use behind your own lock
    let mut random = RandomGenerator::new();
    println!("Owned random ID: {:032x}", random.next_id());

    // Get an ID with 8 bits of sub-millisecond time fraction in rand_a
    let local_order_id = gen_id_with_sub_ms_8();
    println!("Locally ordered ID: {:032x}", local_order_id);

    // Get ID as canonical string (allocates a String)
    let id_string = gen_id_string();
    println!("Generated ID string: {}", id_string);

    // Get ID as stack-allocated string (no allocation, implements Deref<Target=str>)
    let stack_str = gen_id_str();
    println!("Generated ID stack string: {}", stack_str);

    // Format an existing u128 as 32 hex chars without dashes
    println!("Hex: {}", format_uuid_hex(id));
}
```

### After `fork()`

`fork()` copies the generator state, so a child process would replay its parent's random bits. Call `fast_uuid_v7::reseed()` in the child right after forking. A `SequentialGenerator` or `RandomGenerator` created before the fork is not covered: create a new instance in the child, or call `RandomGenerator::reseed`.

## Performance

Measured with `cargo bench` on an Apple M1:

| Function                           | Time per ID |
|------------------------------------|-------------|
| `gen_id`                           | ~8 ns       |
| `gen_id_with_count`                | ~5 ns       |
| `gen_id_with_sub_ms_8`             | ~7.5 ns     |
| `SequentialGenerator::next_id`     | ~5 ns       |
| `gen_id_str` (no allocation)       | ~11 ns      |
| `gen_id_with_count_str`            | ~11 ns      |
| `SequentialGenerator::next_id_str` | ~9 ns       |
| `gen_id_string` (heap allocation)  | ~66 ns      |
| `format_uuid` / `format_uuid_hex`  | ~7 / ~3 ns  |
| `uuid::Uuid::now_v7()` (reference) | ~1.43 µs    |

Generating 10 million IDs with `gen_id` takes approximately **70ms** on a single core.

### How is it so fast?

1.  **No locks on the hot path**: The `gen_id*` functions (including `gen_id_with_count`) use thread-local state, so there are no mutexes and no shared atomics per call. `SequentialGenerator` and `RandomGenerator` own their state and take `&mut self` instead; any synchronization is up to the caller. The only shared state is one-time setup (CPU counter calibration, CPU feature detection).
2.  **Amortized Time Reads**: Reading wall-clock time is still more expensive than reading a CPU counter, so we avoid doing it on every call.
3.  **Hardware Counters**: On `x86_64` (`rdtsc`) and `aarch64` (`cntvct_el0`) we use CPU counters to cheaply detect when the next millisecond boundary should have been reached, and only then refresh the wall-clock timestamp. Other architectures read `SystemTime::now()` on every call.
4.  **SmallRng**: Uses a fast, non-cryptographic pseudo-random number generator, seeded from the OS.
5.  **SIMD formatting**: Hex formatting uses NEON on `aarch64` and SSSE3 on x86 (detected at runtime), with a scalar fallback.
6.  **Stack Allocation**: `gen_id_str` and `format_uuid` write into a stack buffer, avoiding `malloc`.

### Limitations

*   **Not Cryptographically Secure**: The randomness is optimized for speed, not unpredictability. Do not use for session tokens or secrets. If you don't need speed, use the original `uuid` crate.
*   **Monotonicity**: Only guaranteed per-thread if using `gen_id_with_count`, or per-instance with `SequentialGenerator`. Otherwise, IDs within the same millisecond are random.
*   **Fork safety**: After `fork()`, call `reseed()` in the child, see [After `fork()`](#after-fork).
*   **Sub-millisecond fractions are approximate**: the `gen_id_with_sub_ms_*` variants improve intra-millisecond sort locality, but the extra bits are often estimated rather than freshly measured and are not a true higher-resolution timestamp.
*   **Clock Drift Risk**: The batched timestamp check assumes the CPU counter frequency is stable. While we include safety checks, extreme edge cases (e.g., VM migration) might cause a 1ms timestamp lag.
*   **Still needs wall-clock reads**: The fastest path only happens while we can reuse the last millisecond timestamp. Once the next millisecond boundary is due, we still need to refresh from `SystemTime::now()`, so actual throughput depends on workload and platform details.

### Benchmarking

To check performance on your machine:

```bash
# measure time to generate 10 million ids:
cargo test --release -- test_next_id_performance --nocapture
# or - for the alternative bench test that measures the time per ID generation
cargo bench 
```

## Python package

A Python wrapper lives in [python/README.md](python/README.md).

It uses PyO3 and maturin to expose the Rust generator as a `fastuuidv7` module for local speed experiments and for older Python versions. Python 3.14 already includes `uuid.uuid7()` in the standard library.

## Node.js / Deno / Bun package

A napi-rs binding lives in [node/README.md](node/README.md) and is published to npm as `fast-uuid-v7`. On Node.js it generates a UUID v7 string in ~45ns, about 13x faster than the `uuidv7` npm package and ~40x faster than `uuid`.

## Disclaimer

This generator is designed for database keys and sorting, not for cryptography.
The random component is optimized for speed, not unpredictability.


## License

MIT
