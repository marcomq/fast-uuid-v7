# fast-uuid-v7

A high-performance Rust library for generating UUID v7 compatible identifiers.

This implementation focuses on speed. It uses thread-local storage and a seeded `SmallRng` to generate IDs without lock contention, making it suitable for high-throughput applications.

## Features

*   **UUID v7**: Time-ordered, 128-bit unique identifiers.
*   **Fast**: Minimal overhead using thread-local state.
*   **Flexible**: Choose between maximum randomness (`gen_id`) or per-thread monotonicity (`gen_id_with_count`).

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

Compared to the standard `uuid` crate (which may take up to ~1.4µs / 1400ns per ID):
*   **`fast-uuid-v7` can be up to ~165x faster** (8.4ns vs 1400ns).
*   When using feature `fast-rng` on the original `uuid` crate, `fast-uuid-v7` can still be up to 
10 times faster for `uint128` (8.4ns vs 90ns) and 8 times faster for `&str` generation (21.5ns vs 170ns).

## Randomness vs Monotonicity

### `gen_id` (74 bits randomness)
The default `gen_id` (and `gen_id_u128`) uses all available 74 bits for randomness. This matches the standard UUID v7 randomness layout.
*   **Pros**: Extremely low collision risk across distributed systems.
*   **Cons**: IDs generated within the same millisecond on the same thread are not guaranteed to be monotonic (they will be random).

### `gen_id_with_count` (56 bits randomness + 18-bit counter)
The `gen_id_with_count` function uses an 18-bit counter and 56 bits of randomness.
*   **Pros**: Guarantees monotonicity per thread (up to ~262k IDs/ms).
*   **Cons**: Reduced randomness (56 bits) increases collision risk in massive distributed systems (approx. 50% chance after 4.5 billion IDs/ms globally).

## Bit Layout

The 128-bit ID is fully compatible with UUID v7. It is composed of:

*   **48 bits**: Unix timestamp in milliseconds.
*   **4 bits**: Version (7).
*   **12 bits**: Random Data (or Counter High 12 bits with `gen_id_with_count`).
*   **2 bits**: Variant (10xx).
*   **62 bits**: Random Data (or Counter Low 6 bits + 56 bits random with `gen_id_with_count`).

**Total Randomness:**
*   `gen_id`: **74 bits**
*   `gen_id_with_count`: **56 bits**

## Usage

```rust
use fast_uuid_v7::{gen_id, gen_id_string, gen_id_str, gen_id_with_count};

fn main() {
    // Get ID as u128 (74 bits random), takes about 8-50ns
    let id = gen_id();
    println!("Generated ID: {:032x}", id);

    // Get monotonic ID (56 bits random + 18-bit counter)
    let ordered_id = gen_id_with_count();
    println!("Ordered ID: {:032x}", ordered_id);

    // Get ID as canonical string (allocates String, takes about 85-130ns)
    let id_string = gen_id_string();
    println!("Generated ID string: {}", id_string);

    // Get ID as stack-allocated string (zero allocation, implements Deref<Target=str>, takes about 21-60ns)
    let stack_str = gen_id_str();
    println!("Generated ID stack string: {}", stack_str);
}
```

## Performance

On a modern machine (e.g., Apple M1 or recent x86_64), you can expect:

*   **`gen_id`**: ~8-50 ns
*   **`gen_id_str`**: ~21-60 ns (zero-allocation)
*   **`gen_id_string`**: ~85-130 ns (includes heap allocation)

Generating 10 million IDs takes approximately **95ms** on a single core.

### How is it so fast?

1.  **Thread-Local Storage**: No mutexes or atomic contention. Each thread has its own state and counters.
2.  **Amortized Syscalls**: `SystemTime::now()` is expensive (~20-40ns). We use the internal CPU clock/tick (if available) to check for time passage, calling the actual system time only periodically.
3.  **Hardware Counters**: To prevent clock drift during the batched calls, we use CPU cycle counters (`rdtsc` on x86, `cntvct_el0` on ARM) to detect thread sleeps or long pauses cheaply.
4.  **SmallRng**: Uses a fast, non-cryptographic pseudo-random number generator.
5.  **Stack Allocation**: `gen_id_str` formats the UUID directly into a stack buffer, avoiding `malloc`.

### Limitations

*   **Not Cryptographically Secure**: The randomness is optimized for speed, not unpredictability. Do not use for session tokens or secrets. If you don't need speed, use the original `uuid` crate.
*   **Monotonicity**: Only guaranteed per-thread if using `gen_id_with_count`. Otherwise, IDs within the same millisecond are random.
*   **Clock Drift Risk**: The batched timestamp check assumes the CPU counter frequency is stable. While we include safety checks, extreme edge cases (e.g., VM migration) might cause a 1ms timestamp lag.
*   **Still needs SystemTime::now()**: The speed of 8ns is not constant and can only be achieved if we can skip calling `SystemTime::now()`. We still need to call `SystemTime::now()` from time to time, for example if the previous call was 1ms ago. In that case, we still need to call `SystemTime::now()` and the performance drops to about 50ns. This is still much faster than the original `uuid` crate.

### Benchmarking

To check performance on your machine:

```bash
# measure time to generate 10 million ids:
cargo test --release -- test_next_id_performance --nocapture
# or - for the alternative bench test that measures the time per ID generation
cargo bench 
```

## Disclaimer

This generator is designed for database keys and sorting, not for cryptography.
The random component is optimized for speed, not unpredictability.


## License

MIT
