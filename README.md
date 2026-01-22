# fast-uuid-v7

A high-performance Rust library for generating UUID v7 compatible identifiers.

This implementation focuses on speed. It uses thread-local storage and a seeded `SmallRng` to generate IDs without lock contention, making it suitable for high-throughput applications.

## Features

*   **UUID v7**: Time-ordered, 128-bit unique identifiers.
*   **Fast**: Minimal overhead using thread-local state.
*   **Monotonic**: IDs generated on the same thread increase monotonically. It supports up to ~262k IDs per millisecond before incrementing the timestamp to preserve order.

## Comparison to `uuid` crate

Compared to the standard `uuid` crate (which may take up to ~1.4µs / 1440ns per ID):
*   **`fast-uuid-v7` can be up to ~130x faster** (11ns vs 1440ns).

As the potential throughput is much higher, the internal counter was increased
from 12 bit to 18 bit, the actual random part was reduced from 64 to 56 bit.

## Bit Layout

The 128-bit ID is still compatible to uuid v7. It is composed of:

*   **48 bits**: Unix timestamp in milliseconds.
*   **4 bits**: Version (7).
*   **12 bits**: Counter (High 12 bits).
*   **2 bits**: Variant (2).
*   **6 bits**: Counter (Low 6 bits).
*   **56 bits**: Random data.

## Usage

```rust
use fast_uuid_v7::{gen_id_u128, gen_id_string, gen_id_str};

fn main() {
    // Get ID as u128, takes about 10ns
    let id = gen_id_u128();
    println!("Generated ID: {:032x}", id);

    // Get ID as canonical string (allocates String, takes about 90ns)
    let id_string = gen_id_string();
    println!("Generated ID string: {}", id_string);

    // Get ID as stack-allocated string (zero allocation, implements Deref<Target=str>, takes about 20ns)
    let stack_str = gen_id_str();
    println!("Generated ID stack string: {}", stack_str);
}
```

## Performance

On a modern machine (e.g., Apple M1 or recent x86_64), you can expect:

*   **`gen_id_u128`**: ~11 ns
*   **`gen_id_str`**: ~23 ns (zero-allocation)
*   **`gen_id_string`**: ~90 ns (includes heap allocation)

Generating 10 million IDs takes approximately **113ms** on a single core.

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
*   **Sometimes slow**: The high speed can only be achieved if we can reduce the amounts of `SystemTime::now()` calls. We still need to call `SystemTime::now()` if the previous call is 1ms ago. In that case, the performance drops to about 40ns per call, which is still much faster than the original `uuid` crate.

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
Also, there is no guaranteed order.



## License

MIT
