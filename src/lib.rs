//  fast-uuid-v7
//  © Copyright 2026, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/fast-uuid-v7

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use std::cell::{Cell, RefCell};
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_rng(&mut rand::rng()));
    static LAST_MS: Cell<u64> = const { Cell::new(0) };
    static COUNTER: Cell<u32> = const { Cell::new(0) };
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    static LAST_TSC: Cell<u64> = const { Cell::new(0) };
}

/// Generates a unique identifier compatible with UUID v7.
///
/// The identifier is a `u128` value composed of:
/// - 48 bits: Current timestamp in milliseconds.
/// -  4 bits: Version (7).
/// - 12 bits: Random data.
/// -  2 bits: Variant (10..).
/// - 62 bits: Random data.
///
/// **Randomness:**
/// This function uses 74 bits of randomness. This provides extremely low
/// collision probability across distributed systems but does not guarantee monotonicity
/// for IDs generated within the same millisecond on the same thread.
///
/// fast-uuid-v7 is is not random enough for cryptography!
#[inline]
pub fn gen_id_u128() -> u128 {
    let (timestamp, _) = get_timestamp_and_counter();

    let timestamp_part = (timestamp as u128) << 80;
    let version_part = 7u128 << 76; // Version 7 (0111)
    let variant_part = 2u128 << 62; // Variant 1 (10..), RFC 4122

    // We need 74 bits of randomness. SmallRng generates 64 bits per call.
    let (r1, r2) = RNG.with(|random_nr| {
        let mut rng = random_nr.borrow_mut();
        (rng.next_u64(), rng.next_u64())
    });

    // rand_a: 12 bits (from r1)
    let rand_a = (r1 & 0xFFF) as u128;
    // rand_b: 62 bits (from r2)
    let rand_b = (r2 & 0x3FFFFFFFFFFFFFFF) as u128;

    timestamp_part | version_part | (rand_a << 64) | variant_part | rand_b
}

/// Alias for `gen_id_u128`.
#[inline]
pub fn gen_id() -> u128 {
    gen_id_u128()
}

/// Generates a UUID v7 string using the `gen_id_u128` function.
///
/// The returned string is in the format `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
///
/// **Note on Sorting:**
/// Since the counter is thread-local and resets every millisecond, IDs generated
/// concurrently by multiple threads within the same millisecond are not guaranteed
/// to be globally monotonic.
///
/// This is not random enough for cryptography!
pub fn gen_id_string() -> String {
    gen_id_str().to_string()
}

/// Generates a UUID v7 string on the stack, avoiding heap allocation.
pub fn gen_id_str() -> UuidString {
    format_uuid(gen_id_u128())
}

/// Formats a u128 UUID into a stack-allocated string representation.
/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
pub fn format_uuid(id: u128) -> UuidString {
    let mut out = UuidString([0; 36]);
    let bytes = id.to_be_bytes();
    const HEX: &[u8; 16] = b"0123456789abcdef";

    unsafe {
        let ptr = out.0.as_mut_ptr();

        // Group 1: 8 chars (4 bytes)
        *ptr.add(0) = HEX[(bytes[0] >> 4) as usize];
        *ptr.add(1) = HEX[(bytes[0] & 0xf) as usize];
        *ptr.add(2) = HEX[(bytes[1] >> 4) as usize];
        *ptr.add(3) = HEX[(bytes[1] & 0xf) as usize];
        *ptr.add(4) = HEX[(bytes[2] >> 4) as usize];
        *ptr.add(5) = HEX[(bytes[2] & 0xf) as usize];
        *ptr.add(6) = HEX[(bytes[3] >> 4) as usize];
        *ptr.add(7) = HEX[(bytes[3] & 0xf) as usize];
        *ptr.add(8) = b'-';

        // Group 2: 4 chars (2 bytes)
        *ptr.add(9) = HEX[(bytes[4] >> 4) as usize];
        *ptr.add(10) = HEX[(bytes[4] & 0xf) as usize];
        *ptr.add(11) = HEX[(bytes[5] >> 4) as usize];
        *ptr.add(12) = HEX[(bytes[5] & 0xf) as usize];
        *ptr.add(13) = b'-';

        // Group 3: 4 chars (2 bytes)
        *ptr.add(14) = HEX[(bytes[6] >> 4) as usize];
        *ptr.add(15) = HEX[(bytes[6] & 0xf) as usize];
        *ptr.add(16) = HEX[(bytes[7] >> 4) as usize];
        *ptr.add(17) = HEX[(bytes[7] & 0xf) as usize];
        *ptr.add(18) = b'-';

        // Group 4: 4 chars (2 bytes)
        *ptr.add(19) = HEX[(bytes[8] >> 4) as usize];
        *ptr.add(20) = HEX[(bytes[8] & 0xf) as usize];
        *ptr.add(21) = HEX[(bytes[9] >> 4) as usize];
        *ptr.add(22) = HEX[(bytes[9] & 0xf) as usize];
        *ptr.add(23) = b'-';

        // Group 5: 12 chars (6 bytes)
        *ptr.add(24) = HEX[(bytes[10] >> 4) as usize];
        *ptr.add(25) = HEX[(bytes[10] & 0xf) as usize];
        *ptr.add(26) = HEX[(bytes[11] >> 4) as usize];
        *ptr.add(27) = HEX[(bytes[11] & 0xf) as usize];
        *ptr.add(28) = HEX[(bytes[12] >> 4) as usize];
        *ptr.add(29) = HEX[(bytes[12] & 0xf) as usize];
        *ptr.add(30) = HEX[(bytes[13] >> 4) as usize];
        *ptr.add(31) = HEX[(bytes[13] & 0xf) as usize];
        *ptr.add(32) = HEX[(bytes[14] >> 4) as usize];
        *ptr.add(33) = HEX[(bytes[14] & 0xf) as usize];
        *ptr.add(34) = HEX[(bytes[15] >> 4) as usize];
        *ptr.add(35) = HEX[(bytes[15] & 0xf) as usize];
    }
    out
}

/// A stack-allocated string representation of a UUID (36 bytes).
///
/// This type implements `Deref<Target=str>`, so it can be used like a `&str`.
/// It avoids heap allocation, making it faster than `gen_id_string`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UuidString([u8; 36]);

impl std::ops::Deref for UuidString {
    type Target = str;
    fn deref(&self) -> &str {
        // SAFETY: The buffer is always filled with valid ASCII (hex + dashes)
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl AsRef<str> for UuidString {
    fn as_ref(&self) -> &str {
        self
    }
}

impl PartialEq<str> for UuidString {
    fn eq(&self, other: &str) -> bool {
        &**self == other
    }
}

impl PartialEq<&str> for UuidString {
    fn eq(&self, other: &&str) -> bool {
        &**self == *other
    }
}

impl std::fmt::Display for UuidString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}

/// Get time every X CPU ticks or every 32 calls
#[inline(always)]
fn get_timestamp_and_counter() -> (u64, u32) {
    // 32 is a good compromise between speed and accuracy
    const TIME_CHECK_MASK: u32 = 0x1F;

    COUNTER.with(|counter_cell| {
        LAST_MS.with(|last_ms_cell| {
            let c = counter_cell.get();
            let last_timestamp = last_ms_cell.get();

            // Check time if counter is 0 (new sequence) or mask matches
            let should_check = {
                #[cfg(target_arch = "x86_64")]
                {
                    // 10,000 cycles is ~3-5us on modern CPUs (>2GHz).
                    // This allows for high throughput while detecting short sleeps.
                    const TSC_THRESHOLD: u64 = 10_000;
                    // SAFETY: _rdtsc is available on x86_64
                    let current_tsc = unsafe { std::arch::x86_64::_rdtsc() };
                    let last_tsc = LAST_TSC.with(|t| t.get());
                    LAST_TSC.with(|t| t.set(current_tsc));
                    (c & TIME_CHECK_MASK) == 0 || current_tsc.wrapping_sub(last_tsc) > TSC_THRESHOLD
                }
                #[cfg(target_arch = "aarch64")]
                {
                    // ARM counter frequency varies (often 1-50MHz).
                    // 1,000 ticks guarantees <1ms lag even on slow 1MHz counters.
                    const TSC_THRESHOLD: u64 = 1_000;
                    let current_tsc: u64;
                    // SAFETY: reading cntvct_el0 is safe in userspace
                    unsafe {
                        std::arch::asm!("mrs {}, cntvct_el0", out(reg) current_tsc, options(nomem, nostack, preserves_flags));
                    }
                    let last_tsc = LAST_TSC.with(|t| t.get());
                    LAST_TSC.with(|t| t.set(current_tsc));
                    (c & TIME_CHECK_MASK) == 0 || current_tsc.wrapping_sub(last_tsc) > TSC_THRESHOLD
                }
                #[cfg(target_arch = "wasm32")]
                {
                    // WASM lacks a cheap cycle counter (like rdtsc) to detect thread sleeps.
                    // We must check the time on every call to prevent timestamp drift.
                    true
                }
                #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "wasm32")))]
                {
                    true
                }
            };

            let mut current_timestamp = if should_check {
                system_time_ms()
            } else {
                last_timestamp
            };

            if current_timestamp > last_timestamp {
                last_ms_cell.set(current_timestamp);
                counter_cell.set(0);
                (current_timestamp, 0)
            } else {
                // Time hasn't moved forward (or we skipped checking).
                current_timestamp = last_timestamp;

                // If counter is exhausted (18 bits = 262,143), increment timestamp to preserve monotonicity
                if c >= 0x3FFFF {
                    current_timestamp += 1;
                    last_ms_cell.set(current_timestamp);
                    counter_cell.set(0);
                    (current_timestamp, 0)
                } else {
                    let inc = c.wrapping_add(1);
                    counter_cell.set(inc);
                    (last_timestamp, inc)
                }
            }
        })
    })
}

/// Returns the current time in milliseconds since the Unix epoch.
///
/// It returns `0` if the system clock hasn't started yet.
#[inline]
fn system_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Generates a UUID v7 with an 18-bit monotonic counter and 56 bits of randomness.
///
/// This guarantees per-thread monotonicity (up to ~262k IDs/ms) but has higher
/// collision risk across different nodes if the random part is exhausted.
#[inline]
pub fn gen_id_with_count() -> u128 {
    let (timestamp, counter) = get_timestamp_and_counter();

    let timestamp_part = (timestamp as u128) << 80;
    let version_part = 7u128 << 76; // Version 7 (0111)
    let variant_part = 2u128 << 62; // Variant 1 (10..), RFC 4122

    // Use 18 bits for counter: 12 in rand_a, 6 in rand_b high.
    let rand_a = (counter >> 6) & 0xFFF;
    let rand_b_high = counter & 0x3F;

    let rand_nr = RNG.with(|random_nr| random_nr.borrow_mut().next_u64());

    let counter_part = (rand_a as u128) << 64; // 12 bits of counter
                                               // 56 bits of randomness + 6 bits of counter
    let rand_b_low = rand_nr & 0x00FF_FFFF_FFFF_FFFF;
    let random_part = ((rand_b_high as u128) << 56) | (rand_b_low as u128);

    timestamp_part | version_part | counter_part | variant_part | random_part
}

#[inline]
pub fn gen_id_with_count_str() -> UuidString {
    format_uuid(gen_id_with_count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// test with `cargo test --release -- test_next_id_performance --nocapture`
    fn test_next_id_performance() {
        let start = std::time::Instant::now();
        for _ in 0..10_000_000 {
            let _ = gen_id_u128();
        }
        println!("Generated 10,000,000 IDs in {:?}", start.elapsed());
    }

    #[test]
    fn test_next_id_uniqueness() {
        let mut set = std::collections::HashSet::with_capacity(1_000_000);
        for _ in 0..1_000_000 {
            let id = gen_id_u128();
            assert!(set.insert(id), "Duplicate ID generated: {:032x}", id);
        }
    }

    #[test]
    /// IDs are sorted correctly per thread.
    /// Capacity is ~262k IDs per ms (18 bits).
    fn test_next_id_ordering() {
        let mut last_id = 0;
        for _ in 0..1_000_000 {
            let id = gen_id_with_count();
            if last_id != 0 {
                assert!(
                    id > last_id,
                    "IDs are not ordered: {:032x} <= {:032x}",
                    id,
                    last_id
                );
            }
            last_id = id;
        }
    }

    #[test]
    fn test_next_id_string() {
        let id_str = gen_id_string();
        assert_eq!(id_str.len(), 36);
        assert!(uuid::Uuid::parse_str(&id_str).is_ok());
    }

    #[test]
    fn test_format_uuid_correctness() {
        let id = gen_id_u128();
        let formatted = format_uuid(id);
        let uuid_crate_str = uuid::Uuid::from_u128(id).to_string();
        assert_eq!(formatted.as_ref(), uuid_crate_str);
    }

    #[test]
    fn test_gen_id_structure() {
        let id = gen_id();
        let uuid = uuid::Uuid::from_u128(id);
        assert_eq!(uuid.get_version(), Some(uuid::Version::SortRand));
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn test_gen_id_with_count_structure() {
        let id = gen_id_with_count();
        let uuid = uuid::Uuid::from_u128(id);
        assert_eq!(uuid.get_version(), Some(uuid::Version::SortRand));
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
    }
}
