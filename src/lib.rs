//  fast-uuid-v7
//  © Copyright 2026, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/fast-uuid-v7

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};

struct ThreadState {
    rng: SmallRng,
    last_ms: u64,
    counter: u32,
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    last_tsc: u64,
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    threshold: u64,
}

impl ThreadState {
    fn new() -> Self {
        let rng = SmallRng::from_rng(&mut rand::rng());

        #[cfg(target_arch = "x86_64")]
        {
            // Default to 2GHz (2000 MHz) if detection fails
            let mut base_mhz = 2000;
            // SAFETY: cpuid is safe on x86_64
            unsafe {
                let max_leaf = std::arch::x86_64::__get_cpuid_max(0).0;
                if max_leaf >= 0x16 {
                    let res = std::arch::x86_64::__cpuid(0x16);
                    if res.eax > 0 {
                        base_mhz = res.eax as u64;
                    }
                }
            }
            // Threshold for ~0.1ms
            let threshold = base_mhz * 100;
            Self {
                rng,
                last_ms: 0,
                counter: 0,
                last_tsc: 0,
                threshold,
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            let freq: u64;
            // SAFETY: reading cntfrq_el0 is safe in userspace on Linux/macOS
            unsafe {
                std::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq, options(nomem, nostack, preserves_flags));
            }
            let threshold = freq / 10000; // 0.1ms
            Self {
                rng,
                last_ms: 0,
                counter: 0,
                last_tsc: 0,
                threshold,
            }
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Self {
                rng,
                last_ms: 0,
                counter: 0,
            }
        }
    }

    #[inline(always)]
    fn should_check_time(&mut self) -> bool {
        // We want to avoid calling SystemTime::now() (expensive) on every call.
        // We force a check if:
        // A significant amount of CPU time has passed since the last call (latency/sleep detection).
        // This prevents using an old timestamp if the thread slept but the counter didn't wrap.
        #[cfg(target_arch = "x86_64")]
        {
            // SAFETY: _rdtsc is available on x86_64
            let current_tsc = unsafe { std::arch::x86_64::_rdtsc() };
            if current_tsc.wrapping_sub(self.last_tsc) > self.threshold {
                self.last_tsc = current_tsc;
                true
            } else {
                false
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            let current_tsc: u64;
            // SAFETY: reading cntvct_el0 is safe in userspace
            unsafe {
                std::arch::asm!("mrs {}, cntvct_el0", out(reg) current_tsc, options(nomem, nostack, preserves_flags));
            }
            if current_tsc.wrapping_sub(self.last_tsc) > self.threshold {
                self.last_tsc = current_tsc;
                true
            } else {
                false
            }
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            // WASM/others lack a cheap cycle counter. Check every call to be safe.
            true
        }
    }

    #[inline(always)]
    fn get_time(&mut self) -> u64 {
        if self.should_check_time() {
            let now = system_time_ms();
            if now > self.last_ms {
                self.last_ms = now;
                self.counter = 0;
            }
        }
        self.last_ms
    }

    #[inline(always)]
    fn get_time_and_counter(&mut self) -> (u64, u32) {
        let should_check = self.should_check_time();

        let mut current_timestamp = if should_check {
            system_time_ms()
        } else {
            self.last_ms
        };

        if current_timestamp > self.last_ms {
            self.last_ms = current_timestamp;
            self.counter = 0;
            (current_timestamp, 0)
        } else {
            // Time hasn't moved forward (or we skipped checking).
            current_timestamp = self.last_ms;
            let c = self.counter;

            // If counter is exhausted (18 bits = 262,143), increment timestamp to preserve monotonicity
            if c >= 0x3FFFF {
                current_timestamp += 1;
                self.last_ms = current_timestamp;
                self.counter = 0;
                (current_timestamp, 0)
            } else {
                let inc = c.wrapping_add(1);
                self.counter = inc;
                (current_timestamp, inc)
            }
        }
    }
}

thread_local! {
    static STATE: RefCell<ThreadState> = RefCell::new(ThreadState::new());
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
    STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        let timestamp = state.get_time();

        let timestamp_part = (timestamp as u128) << 80;
        let version_part = 7u128 << 76; // Version 7 (0111)
        let variant_part = 2u128 << 62; // Variant 1 (10..), RFC 4122

        // We need 74 bits of randomness. SmallRng generates 64 bits per call.
        let r1 = state.rng.next_u32();
        let r2 = state.rng.next_u64();

        // rand_a: 12 bits (from r1)
        let rand_a = (r1 & 0xFFF) as u128;
        // rand_b: 62 bits (from r2)
        let rand_b = (r2 & 0x3FFFFFFFFFFFFFFF) as u128;

        timestamp_part | version_part | (rand_a << 64) | variant_part | rand_b
    })
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
    STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        let (timestamp, counter) = state.get_time_and_counter();

        let timestamp_part = (timestamp as u128) << 80;
        let version_part = 7u128 << 76; // Version 7 (0111)
        let variant_part = 2u128 << 62; // Variant 1 (10..), RFC 4122

        // Use 18 bits for counter: 12 in rand_a, 6 in rand_b high.
        let rand_a = (counter >> 6) & 0xFFF;
        let rand_b_high = counter & 0x3F;

        let rand_nr = state.rng.next_u64();

        let counter_part = (rand_a as u128) << 64; // 12 bits of counter
                                                   // 56 bits of randomness + 6 bits of counter
        let rand_b_low = rand_nr & 0x00FF_FFFF_FFFF_FFFF;
        let random_part = ((rand_b_high as u128) << 56) | (rand_b_low as u128);

        timestamp_part | version_part | counter_part | variant_part | random_part
    })
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

    #[test]
    fn test_timestamp_updates_continuously() {
        let start = std::time::Instant::now();
        let duration = std::time::Duration::from_millis(100);

        let mut last_ts = gen_id() >> 80;
        let start_ts = last_ts;
        let mut distinct_timestamps = 0;

        while start.elapsed() < duration {
            let curr = gen_id();
            let curr_ts = curr >> 80;
            if curr_ts > last_ts {
                distinct_timestamps += 1;
                last_ts = curr_ts;
            }
        }

        let elapsed_ts = last_ts - start_ts;
        println!(
            "Timestamp advanced: {} ms, Distinct timestamps observed: {}",
            elapsed_ts, distinct_timestamps
        );

        // Expect at least 80ms of advancement in 100ms real time.
        // If this fails, it means the time checking logic (TSC threshold) isn't triggering often enough
        // or the system clock is frozen.
        assert!(
            elapsed_ts >= 99,
            "Timestamp should advance roughly 100ms, got {}ms",
            elapsed_ts
        );

        // We should see many millisecond transitions if we are spinning in a loop.
        // If this fails, the thread might have been descheduled for long periods.
        if cfg!(debug_assertions) {
            assert!(
                distinct_timestamps >= 90,
                "Should see frequent updates, got {} distinct timestamps",
                distinct_timestamps
            );
        } else {
            assert!(
                distinct_timestamps >= 99,
                "Should see frequent updates, got {} distinct timestamps",
                distinct_timestamps
            );
        }
    }

    #[test]
    fn test_counter_reset() {
        let start_ts = gen_id_with_count() >> 80;
        loop {
            let id = gen_id_with_count();
            let ts = id >> 80;
            if ts > start_ts {
                let counter_high = (id >> 64) & 0xFFF;
                let counter_low = (id >> 56) & 0x3F;
                let counter = (counter_high << 6) | counter_low;

                assert_eq!(
                    counter, 0,
                    "Counter should reset to 0 when timestamp changes"
                );
                break;
            }
        }
    }
}
