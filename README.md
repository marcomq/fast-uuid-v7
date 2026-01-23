# fast-uuid-v7

A high-performance Rust library for generating UUID v7 compatible identifiers.

This implementation focuses on speed. It uses thread-local storage and a seeded `SmallRng` to generate IDs without lock contention, making it suitable for high-throughput applications.

## Features

*   **UUID v7**: Time-ordered, 128-bit unique identifiers.
*   **Fast**: Minimal overhead using thread-local state.
*   **Monotonic**: IDs generated on the same thread increase monotonically. It supports up to ~262k IDs per millisecond before incrementing the timestamp to preserve order.

## Why?

I was testing the performance of network streams and used the original `uuid` v7 to generate messages
with unique IDs. I wondered why there was a maximum of 400,000 messages per 
second, even though the rest of the code looked like it could handle millions.
I figured out that 400,000 is mostly the limit of the standard `uuid` crate when
using v7. Even v4 was not significantly faster. This happened due to the strong
random number generator used in `uuid`. While a strong RNG is correct and avoids potential issues
in security or crypto contexts, it is otherwise just very slow.

## Comparison to `uuid` crate

Compared to the standard `uuid` crate (which may take up to ~1.4µs / 1440ns per ID):
*   **`fast-uuid-v7` can be up to ~130x faster** (11ns vs 1440ns).

As the potential throughput is much higher, the internal counter was increased
from 12 bits to 18 bits, and the random part was reduced from 64 bits to 56 bits.

## Bit Layout

The 128-bit ID is fully compatible with UUID v7. It is composed of:

*   **48 bits**: Unix timestamp in milliseconds.
*   **4 bits**: Version (7).
*   **12 bits**: Counter (High 12 bits).
*   **2 bits**: Variant (10xx).
*   **6 bits**: Counter (Low 6 bits).
*   **56 bits**: Random data.

## Usage

```rust
use fast_uuid_v7::{gen_id, gen_id_string, gen_id_str};

fn main() {
    // Get ID as u128, takes about 11-50ns
    let id = gen_id();
    println!("Generated ID: {:032x}", id);

    // Get ID as canonical string (allocates String, takes about 90-130ns)
    let id_string = gen_id_string();
    println!("Generated ID string: {}", id_string);

    // Get ID as stack-allocated string (zero allocation, implements Deref<Target=str>, takes about 20-60ns)
    let stack_str = gen_id_str();
    println!("Generated ID stack string: {}", stack_str);
}
```

## Performance

On a modern machine (e.g., Apple M1 or recent x86_64), you can expect:

*   **`gen_id_u128`**: ~11 ns
*   **`gen_id_str`**: ~23 ns (zero-allocation)
*   **`gen_id_string`**: ~90 ns (includes heap allocation)

Generating 10 million IDs takes approximately **120ms** on a single core.

### How is it so fast?

1.  **Thread-Local Storage**: No mutexes or atomic contention. Each thread has its own state and counters.
2.  **Amortized Syscalls**: `SystemTime::now()` is expensive (~20-40ns). We use the internal CPU clock/tick (if available) to check for time passage, calling the actual system time only periodically.
3.  **Hardware Counters**: To prevent clock drift during the batched calls, we use CPU cycle counters (`rdtsc` on x86, `cntvct_el0` on ARM) to detect thread sleeps or long pauses cheaply.
4.  **SmallRng**: Uses a fast, non-cryptographic pseudo-random number generator.
5.  **Stack Allocation**: `gen_id_str` formats the UUID directly into a stack buffer, avoiding `malloc`.

### Limitations

*   **Not Cryptographically Secure**: The randomness is optimized for speed, not unpredictability. Do not use for session tokens or secrets. If you don't need speed, use the original `uuid` crate.
*   **Per-Thread Monotonicity**: IDs are monotonic within a single thread. Across threads, they are only roughly ordered by timestamp (1ms precision).
*   **Clock Drift Risk**: The batched timestamp check assumes the CPU counter frequency is stable. While we include safety checks, extreme edge cases (e.g., VM migration) might cause a 1ms timestamp lag.
*   **Still needs SystemTime::now()**: The speed of 11ns is not constant and can only be achieved if we can skip calling `SystemTime::now()`. We still need to call `SystemTime::now()` from time to time, for example if the previous call was 1ms ago. In that case, we still need to call `SystemTime::now()` and the performance drops to about 50ns. This is still much faster than the original `uuid` crate.

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
