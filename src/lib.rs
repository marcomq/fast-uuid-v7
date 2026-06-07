//  fast-uuid-v7
//  © Copyright 2026, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/fast-uuid-v7

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use std::cell::RefCell;
#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    not(target_feature = "ssse3")
))]
use std::sync::atomic::{AtomicU8, Ordering};

const COUNTER_MAX: u32 = 0x3FFFF;
const COUNTER_SEED_MASK: u32 = 0x0FFF;
const HEX: &[u8; 16] = b"0123456789abcdef";
#[cfg(not(target_arch = "aarch64"))]
const HEX_PAIRS: [[u8; 2]; 256] = hex_pairs();
#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    not(target_feature = "ssse3")
))]
static X86_FORMATTER: AtomicU8 = AtomicU8::new(0);

#[cfg(not(target_arch = "aarch64"))]
const fn hex_pairs() -> [[u8; 2]; 256] {
    let mut pairs = [[0u8; 2]; 256];
    let mut i = 0;

    while i < 256 {
        pairs[i][0] = HEX[i >> 4];
        pairs[i][1] = HEX[i & 0x0f];
        i += 1;
    }

    pairs
}

#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    not(target_feature = "ssse3")
))]
#[inline(always)]
fn x86_has_ssse3() -> bool {
    match X86_FORMATTER.load(Ordering::Relaxed) {
        2 => true,
        1 => false,
        _ => {
            let has_ssse3 = std::arch::is_x86_feature_detected!("ssse3");
            X86_FORMATTER.store(if has_ssse3 { 2 } else { 1 }, Ordering::Relaxed);
            has_ssse3
        }
    }
}

mod clock;

struct ThreadState {
    rng: SmallRng,
    last_ms: u64,
    last_nanos_within_ms: u32,
    last_sampled_nanos_within_ms: u32,
    counter: u32,
    clock: clock::Clock,
}

impl ThreadState {
    fn new() -> Self {
        Self {
            rng: SmallRng::from_rng(&mut rand::rng()),
            last_ms: 0,
            last_nanos_within_ms: 0,
            last_sampled_nanos_within_ms: 0,
            counter: 0,
            clock: clock::Clock::new(),
        }
    }

    #[inline(always)]
    fn seed_counter(&mut self) -> u32 {
        // RFC 9562 allows seeding only a portion of a fixed-length counter.
        // We randomize the low 12 bits to keep the full 18-bit layout while
        // preserving almost all per-millisecond headroom before rollover.
        self.rng.next_u32() & COUNTER_SEED_MASK
    }

    #[inline(always)]
    fn refresh_time(&mut self) -> bool {
        let sample = self.clock.refresh_timestamp();
        self.record_time_sample(sample, true)
    }

    #[inline(always)]
    fn current_timestamp_sample(&self) -> clock::TimestampSample {
        clock::TimestampSample {
            ms: self.last_ms,
            nanos_within_ms: self.last_nanos_within_ms,
        }
    }

    #[inline(always)]
    fn get_time(&mut self) -> u64 {
        if self.last_ms == 0 || self.clock.should_refresh() {
            self.refresh_time();
        }
        self.last_ms
    }

    #[inline(always)]
    fn get_time_and_counter(&mut self) -> (u64, u32) {
        if (self.last_ms == 0 || self.clock.should_refresh()) && self.refresh_time() {
            (self.last_ms, self.counter)
        } else {
            let c = self.counter;
            let mut current_timestamp = self.last_ms;

            // If counter is exhausted (18 bits = 262,143), increment timestamp to preserve monotonicity
            if c >= COUNTER_MAX {
                current_timestamp += 1;
                self.last_ms = current_timestamp;
                self.last_nanos_within_ms = 0;
                self.last_sampled_nanos_within_ms = 0;
                self.counter = self.seed_counter();
                (current_timestamp, self.counter)
            } else {
                let inc = c.wrapping_add(1);
                self.counter = inc;
                (current_timestamp, inc)
            }
        }
    }

    #[inline(always)]
    fn record_time_sample(&mut self, sample: clock::TimestampSample, refreshed: bool) -> bool {
        if sample.ms > self.last_ms {
            if refreshed {
                self.last_sampled_nanos_within_ms = sample.nanos_within_ms;
            }
            self.last_ms = sample.ms;
            self.last_nanos_within_ms = sample.nanos_within_ms;
            self.counter = self.seed_counter();
            true
        } else if sample.ms == self.last_ms {
            if refreshed {
                self.last_sampled_nanos_within_ms = sample.nanos_within_ms;
            }
            self.last_nanos_within_ms = self.last_nanos_within_ms.max(sample.nanos_within_ms);
            false
        } else {
            false
        }
    }

    #[inline(always)]
    fn sample_time(&mut self) -> clock::TimestampSample {
        let refreshed = self.last_ms == 0 || self.clock.should_refresh();
        let sample = if refreshed {
            self.clock.refresh_timestamp()
        } else {
            self.clock
                .estimated_timestamp(self.last_ms, self.last_sampled_nanos_within_ms)
        };
        self.record_time_sample(sample, refreshed);
        self.current_timestamp_sample()
    }
}

thread_local! {
    static STATE: RefCell<ThreadState> = RefCell::new(ThreadState::new());
}

#[inline]
fn compose_rand_a(random: u16, fraction: u16, bits: u8) -> u16 {
    debug_assert!(bits <= 12);

    let random_bits = 12 - bits;
    let random_mask = if random_bits == 0 {
        0
    } else {
        (1u16 << random_bits) - 1
    };

    ((fraction & ((1u16 << bits) - 1)) << random_bits) | (random & random_mask)
}

#[inline]
fn uuid_v7_from_parts(timestamp_ms: u64, rand_a: u16, rand_b: u64) -> u128 {
    let timestamp_part = (timestamp_ms as u128) << 80;
    let version_part = 7u128 << 76;
    let variant_part = 2u128 << 62;

    timestamp_part
        | version_part
        | ((rand_a as u128) << 64)
        | variant_part
        | ((rand_b & 0x3FFF_FFFF_FFFF_FFFF) as u128)
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

        // We need 74 bits of randomness. SmallRng generates 64 bits per call.
        let r1 = state.rng.next_u32();
        let r2 = state.rng.next_u64();

        // rand_a: 12 bits (from r1)
        let rand_a = (r1 & 0x0FFF) as u16;
        // rand_b: 62 bits (from r2)
        let rand_b = r2 & 0x3FFF_FFFF_FFFF_FFFF;

        uuid_v7_from_parts(timestamp, rand_a, rand_b)
    })
}

/// Alias for `gen_id_u128`.
#[inline]
pub fn gen_id() -> u128 {
    gen_id_u128()
}

/// Generates a UUID v7 with an RFC 9562-style sub-millisecond time fraction.
///
/// The 48-bit UUID timestamp remains milliseconds since the Unix epoch. This
/// API fills the high `bits` of `rand_a` with a scaled fraction of the current
/// millisecond and leaves the remaining random bits unchanged. On supported
/// counter backends, the millisecond timestamp comes from wall-clock time, but
/// the sub-millisecond fraction is often estimated between wall-clock refreshes
/// instead of being freshly measured on every call.
///
/// This can improve sort locality for IDs produced within the same millisecond,
/// but it does not provide true nanosecond ordering or distributed monotonicity.
#[inline]
fn gen_id_with_sub_ms_bits(bits: u8) -> u128 {
    debug_assert!(matches!(bits, 4 | 8 | 12));

    STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        let sample = state.sample_time();

        let r1 = state.rng.next_u32();
        let r2 = state.rng.next_u64();

        let fraction = sample.sub_ms_fraction(bits);
        let rand_a = compose_rand_a((r1 & 0x0FFF) as u16, fraction, bits);
        let rand_b = r2 & 0x3FFF_FFFF_FFFF_FFFF;

        uuid_v7_from_parts(sample.ms, rand_a, rand_b)
    })
}

/// Generates a UUID v7 with a 4-bit sub-millisecond fraction in `rand_a`.
#[inline]
pub fn gen_id_with_sub_ms_4() -> u128 {
    gen_id_with_sub_ms_bits(4)
}

/// Generates a UUID v7 with an 8-bit sub-millisecond fraction in `rand_a`.
#[inline]
pub fn gen_id_with_sub_ms_8() -> u128 {
    gen_id_with_sub_ms_bits(8)
}

/// Generates a UUID v7 with a 12-bit sub-millisecond fraction in `rand_a`.
#[inline]
pub fn gen_id_with_sub_ms_12() -> u128 {
    gen_id_with_sub_ms_bits(12)
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
///
/// The returned [`UuidString`] owns its bytes. Borrow it when passing it to APIs
/// that need `&str`, or call [`UuidString::as_str`] explicitly.
pub fn gen_id_str() -> UuidString {
    format_uuid(gen_id_u128())
}

/// Formats a u128 UUID into a stack-allocated string representation.
/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
pub fn format_uuid(id: u128) -> UuidString {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // SAFETY: SSSE3 is enabled for this compilation unit.
        return unsafe { format_uuid_simd(id) };
    }

    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        not(target_feature = "ssse3")
    ))]
    {
        if x86_has_ssse3() {
            // SAFETY: x86_has_ssse3 verifies SSSE3 support before calling
            // the target-feature-specialized formatter.
            return unsafe { format_uuid_simd(id) };
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: NEON/AdvSIMD is part of the aarch64 baseline.
        uuid_string_from_hex(unsafe { format_uuid_hex_neon(id) })
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        format_uuid_scalar(id)
    }
}

/// Formats a u128 UUID into a stack-allocated lowercase hex representation.
/// `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`
pub fn format_uuid_hex(id: u128) -> UuidHex {
    UuidHex(format_uuid_hex_bytes(id))
}

fn format_uuid_hex_bytes(id: u128) -> [u8; 32] {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // SAFETY: SSSE3 is enabled for this compilation unit.
        return unsafe { format_uuid_hex_simd(id) };
    }

    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        not(target_feature = "ssse3")
    ))]
    {
        if x86_has_ssse3() {
            // SAFETY: x86_has_ssse3 verifies SSSE3 support before calling
            // the target-feature-specialized formatter.
            return unsafe { format_uuid_hex_simd(id) };
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: NEON/AdvSIMD is part of the aarch64 baseline.
        unsafe { format_uuid_hex_neon(id) }
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        format_uuid_hex_scalar(id)
    }
}

#[cfg(not(target_arch = "aarch64"))]
#[inline(always)]
fn format_uuid_scalar(id: u128) -> UuidString {
    let mut out = UuidString([0; 36]);
    let bytes = id.to_be_bytes();

    unsafe {
        let ptr = out.0.as_mut_ptr();

        // Group 1: 8 chars (4 bytes)
        let pair = HEX_PAIRS[bytes[0] as usize];
        *ptr.add(0) = pair[0];
        *ptr.add(1) = pair[1];
        let pair = HEX_PAIRS[bytes[1] as usize];
        *ptr.add(2) = pair[0];
        *ptr.add(3) = pair[1];
        let pair = HEX_PAIRS[bytes[2] as usize];
        *ptr.add(4) = pair[0];
        *ptr.add(5) = pair[1];
        let pair = HEX_PAIRS[bytes[3] as usize];
        *ptr.add(6) = pair[0];
        *ptr.add(7) = pair[1];
        *ptr.add(8) = b'-';

        // Group 2: 4 chars (2 bytes)
        let pair = HEX_PAIRS[bytes[4] as usize];
        *ptr.add(9) = pair[0];
        *ptr.add(10) = pair[1];
        let pair = HEX_PAIRS[bytes[5] as usize];
        *ptr.add(11) = pair[0];
        *ptr.add(12) = pair[1];
        *ptr.add(13) = b'-';

        // Group 3: 4 chars (2 bytes)
        let pair = HEX_PAIRS[bytes[6] as usize];
        *ptr.add(14) = pair[0];
        *ptr.add(15) = pair[1];
        let pair = HEX_PAIRS[bytes[7] as usize];
        *ptr.add(16) = pair[0];
        *ptr.add(17) = pair[1];
        *ptr.add(18) = b'-';

        // Group 4: 4 chars (2 bytes)
        let pair = HEX_PAIRS[bytes[8] as usize];
        *ptr.add(19) = pair[0];
        *ptr.add(20) = pair[1];
        let pair = HEX_PAIRS[bytes[9] as usize];
        *ptr.add(21) = pair[0];
        *ptr.add(22) = pair[1];
        *ptr.add(23) = b'-';

        // Group 5: 12 chars (6 bytes)
        let pair = HEX_PAIRS[bytes[10] as usize];
        *ptr.add(24) = pair[0];
        *ptr.add(25) = pair[1];
        let pair = HEX_PAIRS[bytes[11] as usize];
        *ptr.add(26) = pair[0];
        *ptr.add(27) = pair[1];
        let pair = HEX_PAIRS[bytes[12] as usize];
        *ptr.add(28) = pair[0];
        *ptr.add(29) = pair[1];
        let pair = HEX_PAIRS[bytes[13] as usize];
        *ptr.add(30) = pair[0];
        *ptr.add(31) = pair[1];
        let pair = HEX_PAIRS[bytes[14] as usize];
        *ptr.add(32) = pair[0];
        *ptr.add(33) = pair[1];
        let pair = HEX_PAIRS[bytes[15] as usize];
        *ptr.add(34) = pair[0];
        *ptr.add(35) = pair[1];
    }
    out
}

#[cfg(not(target_arch = "aarch64"))]
#[inline(always)]
fn format_uuid_hex_scalar(id: u128) -> [u8; 32] {
    let bytes = id.to_be_bytes();
    let mut out = [0u8; 32];

    for (idx, byte) in bytes.iter().enumerate() {
        let pair = HEX_PAIRS[*byte as usize];
        out[idx * 2] = pair[0];
        out[idx * 2 + 1] = pair[1];
    }

    out
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "ssse3")]
unsafe fn format_uuid_simd(id: u128) -> UuidString {
    uuid_string_from_hex(unsafe { format_uuid_hex_simd(id) })
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "ssse3")]
unsafe fn format_uuid_hex_simd(id: u128) -> [u8; 32] {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{
        __m128i, _mm_and_si128, _mm_loadu_si128, _mm_set1_epi8, _mm_shuffle_epi8, _mm_srli_epi16,
        _mm_storeu_si128, _mm_unpackhi_epi8, _mm_unpacklo_epi8,
    };
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        __m128i, _mm_and_si128, _mm_loadu_si128, _mm_set1_epi8, _mm_shuffle_epi8, _mm_srli_epi16,
        _mm_storeu_si128, _mm_unpackhi_epi8, _mm_unpacklo_epi8,
    };

    let bytes = id.to_be_bytes();
    let mut hex = [0u8; 32];

    unsafe {
        let raw = _mm_loadu_si128(bytes.as_ptr() as *const __m128i);
        let mask = _mm_set1_epi8(0x0f);
        let table = _mm_loadu_si128(HEX.as_ptr() as *const __m128i);

        let lo = _mm_and_si128(raw, mask);
        let hi = _mm_and_si128(_mm_srli_epi16(raw, 4), mask);
        let nibbles_lo = _mm_unpacklo_epi8(hi, lo);
        let nibbles_hi = _mm_unpackhi_epi8(hi, lo);

        let hex_lo = _mm_shuffle_epi8(table, nibbles_lo);
        let hex_hi = _mm_shuffle_epi8(table, nibbles_hi);

        _mm_storeu_si128(hex.as_mut_ptr() as *mut __m128i, hex_lo);
        _mm_storeu_si128(hex.as_mut_ptr().add(16) as *mut __m128i, hex_hi);
    }

    hex
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn format_uuid_hex_neon(id: u128) -> [u8; 32] {
    use std::arch::aarch64::{
        uint8x16_t, vandq_u8, vdupq_n_u8, vld1q_u8, vqtbl1q_u8, vshrq_n_u8, vst1q_u8, vzip1q_u8,
        vzip2q_u8,
    };

    let bytes = id.to_be_bytes();
    let mut hex = [0u8; 32];

    unsafe {
        let raw = vld1q_u8(bytes.as_ptr());
        let table = vld1q_u8(HEX.as_ptr());
        let mask = vdupq_n_u8(0x0f);

        let lo = vandq_u8(raw, mask);
        let hi = vshrq_n_u8::<4>(raw);
        let nibbles_lo: uint8x16_t = vzip1q_u8(hi, lo);
        let nibbles_hi: uint8x16_t = vzip2q_u8(hi, lo);

        let hex_lo = vqtbl1q_u8(table, nibbles_lo);
        let hex_hi = vqtbl1q_u8(table, nibbles_hi);

        vst1q_u8(hex.as_mut_ptr(), hex_lo);
        vst1q_u8(hex.as_mut_ptr().add(16), hex_hi);
    }

    hex
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
#[inline(always)]
fn uuid_string_from_hex(hex: [u8; 32]) -> UuidString {
    let mut out = UuidString([0; 36]);

    out.0[0..8].copy_from_slice(&hex[0..8]);
    out.0[8] = b'-';
    out.0[9..13].copy_from_slice(&hex[8..12]);
    out.0[13] = b'-';
    out.0[14..18].copy_from_slice(&hex[12..16]);
    out.0[18] = b'-';
    out.0[19..23].copy_from_slice(&hex[16..20]);
    out.0[23] = b'-';
    out.0[24..36].copy_from_slice(&hex[20..32]);

    out
}

/// A stack-allocated string representation of a UUID (36 bytes).
///
/// This type owns its bytes and implements `Deref<Target = str>` and
/// `AsRef<str>`, so it can be borrowed by most APIs that need a string slice.
/// It avoids heap allocation, making it faster than [`gen_id_string`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UuidString([u8; 36]);

/// A stack-allocated hex representation of a UUID without dashes (32 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UuidHex([u8; 32]);

impl UuidString {
    /// Returns this UUID as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: The buffer is always filled with valid ASCII (hex + dashes).
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }

    /// Returns the underlying UUID string bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8; 36] {
        &self.0
    }
}

impl std::ops::Deref for UuidString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for UuidString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for UuidString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for UuidString {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl std::fmt::Display for UuidString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl UuidHex {
    /// Returns this UUID as a lowercase hex string slice without dashes.
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: The buffer is always filled with valid ASCII hex.
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }

    /// Returns the underlying UUID hex bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::ops::Deref for UuidHex {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for UuidHex {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for UuidHex {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Generates a UUID v7 with an RFC-style seeded 18-bit monotonic counter and 56 bits of randomness.
///
/// This guarantees per-thread monotonicity (up to ~262k IDs/ms) but has higher
/// collision risk across different nodes if the random part is exhausted.
#[inline]
pub fn gen_id_with_count() -> u128 {
    STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        let (timestamp, counter) = state.get_time_and_counter();

        // Use 18 bits for counter: 12 in rand_a, 6 in rand_b high.
        let rand_a = ((counter >> 6) & 0x0FFF) as u16;
        let rand_b_high = counter & 0x3F;

        let rand_nr = state.rng.next_u64();

        let rand_b_low = rand_nr & 0x00FF_FFFF_FFFF_FFFF;
        let random_part = ((rand_b_high as u64) << 56) | rand_b_low;

        uuid_v7_from_parts(timestamp, rand_a, random_part)
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
    fn test_deadline_is_reached_at_zero_or_after_deadline() {
        assert!(clock::deadline_reached(10, 0));
        assert!(clock::deadline_reached(10, 10));
        assert!(clock::deadline_reached(11, 10));
        assert!(!clock::deadline_reached(9, 10));
    }

    #[test]
    fn test_deadline_handles_counter_wraparound() {
        assert!(!clock::deadline_reached(u64::MAX - 1, 3));
        assert!(clock::deadline_reached(3, u64::MAX - 1));
    }

    #[test]
    fn test_ticks_until_next_refresh_caps_at_half_millisecond() {
        assert_eq!(clock::ticks_until_next_refresh(1_000, 0), 500);
        assert_eq!(clock::ticks_until_next_refresh(1_000, 250_000), 500);
        assert_eq!(clock::ticks_until_next_refresh(1_000, 500_000), 500);
        assert_eq!(clock::ticks_until_next_refresh(1_000, 750_000), 250);
        assert_eq!(clock::ticks_until_next_refresh(1_000, 999_999), 1);
        assert_eq!(clock::ticks_until_next_refresh(1_000, 1_000_000), 500);
    }

    #[test]
    fn test_ticks_until_next_refresh_never_returns_zero() {
        assert_eq!(clock::ticks_until_next_refresh(0, 0), 1);
        assert_eq!(clock::ticks_until_next_refresh(1, 999_999), 1);
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    #[test]
    fn test_clock_without_counter_backend_always_refreshes() {
        let mut clock = clock::Clock::new();
        assert!(clock.should_refresh());

        clock.record_sample(0, 0);
        assert!(clock.should_refresh());
    }

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
    fn test_format_uuid_hex_correctness() {
        let id = gen_id_u128();
        let formatted = format_uuid_hex(id);
        let uuid_crate_str = uuid::Uuid::from_u128(id).simple().to_string();
        assert_eq!(formatted.as_ref(), uuid_crate_str);
    }

    #[test]
    fn test_uuid_string_str_accessors() {
        fn accepts_str(value: &str) -> usize {
            value.len()
        }

        let formatted = gen_id_str();
        assert_eq!(accepts_str(&formatted), 36);
        assert_eq!(accepts_str(formatted.as_str()), 36);
        assert!(uuid::Uuid::parse_str(formatted.as_str()).is_ok());
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
    fn test_gen_id_with_sub_ms_4_has_uuid_v7_layout() {
        let id = gen_id_with_sub_ms_4();
        let uuid = uuid::Uuid::from_u128(id);
        assert_eq!(uuid.get_version(), Some(uuid::Version::SortRand));
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);

        let rand_a = ((id >> 64) & 0x0FFF) as u16;
        assert!(rand_a <= 0x0FFF);
    }

    #[test]
    fn test_sub_ms_fraction_placement_for_expected_bit_widths() {
        let sample = clock::TimestampSample {
            ms: 0,
            nanos_within_ms: 654_321,
        };
        let random = 0b1010_0110_1101u16;

        for bits in [4, 8, 12] {
            let fraction = sample.sub_ms_fraction(bits);
            let rand_a = compose_rand_a(random, fraction, bits);
            let random_bits = 12 - bits;
            let extracted_fraction = rand_a >> random_bits;

            assert_eq!(extracted_fraction, fraction);
        }
    }

    #[test]
    fn test_remaining_rand_a_bits_stay_random() {
        let sample = clock::TimestampSample {
            ms: 0,
            nanos_within_ms: 789_123,
        };
        let random = 0b1101_0011_1010u16;
        let bits = 8;

        let fraction = sample.sub_ms_fraction(bits);
        let rand_a = compose_rand_a(random, fraction, bits);

        assert_eq!(rand_a & 0x000F, random & 0x000F);
    }

    #[test]
    fn test_exact_millisecond_boundary_sub_ms_fraction_stays_in_last_bucket() {
        let sample = clock::TimestampSample {
            ms: 0,
            nanos_within_ms: 1_000_000,
        };
        let rand_a = compose_rand_a(0, sample.sub_ms_fraction(12), 12);

        assert_eq!(rand_a, 0x0FFF);
    }

    #[test]
    fn test_rand_b_remains_random() {
        let timestamp = 1_748_000_000_000u64;
        let rand_a = 0x0ABCu16;
        let rand_b = 0x2ABC_DEF0_1234_5678u64;

        let id = uuid_v7_from_parts(timestamp, rand_a, rand_b);

        assert_eq!(id >> 80, timestamp as u128);
        assert_eq!((id & 0x3FFF_FFFF_FFFF_FFFF) as u64, rand_b);
    }

    #[test]
    fn test_public_sub_ms_variants_produce_distinct_layout_options() {
        let id4 = gen_id_with_sub_ms_4();
        let id8 = gen_id_with_sub_ms_8();
        let id12 = gen_id_with_sub_ms_12();

        for id in [id4, id8, id12] {
            let uuid = uuid::Uuid::from_u128(id);
            assert_eq!(uuid.get_version(), Some(uuid::Version::SortRand));
            assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
        }
    }

    fn extract_sub_ms_fraction(id: u128, bits: u8) -> (u64, u16) {
        let ms = (id >> 80) as u64;
        let rand_a = ((id >> 64) & 0x0FFF) as u16;
        let fraction = rand_a >> (12 - bits);
        (ms, fraction)
    }

    #[test]
    fn test_sub_ms_variants_mostly_increase_within_same_millisecond() {
        let cases: &[(u8, fn() -> u128)] = &[
            (4, gen_id_with_sub_ms_4),
            (8, gen_id_with_sub_ms_8),
            (12, gen_id_with_sub_ms_12),
        ];

        for &(bits, gen_id) in cases {
            let start = std::time::Instant::now();
            let duration = std::time::Duration::from_millis(25);
            let (mut last_ms, mut last_fraction) = extract_sub_ms_fraction(gen_id(), bits);
            let mut comparisons = 0usize;
            let mut nondecreasing = 0usize;

            while start.elapsed() < duration && comparisons < 256 {
                let (ms, fraction) = extract_sub_ms_fraction(gen_id(), bits);
                if ms == last_ms {
                    comparisons += 1;
                    if fraction >= last_fraction {
                        nondecreasing += 1;
                    }
                }

                last_ms = ms;
                last_fraction = fraction;
            }

            assert!(
                comparisons >= 32,
                "not enough same-millisecond samples were observed for the {bits}-bit variant"
            );
            assert!(
                nondecreasing * 10 >= comparisons * 9,
                "sub-ms fraction for the {bits}-bit variant only moved forward in {nondecreasing}/{comparisons} same-millisecond comparisons"
            );
        }
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

        // Expect at least ~100ms of advancement in 100ms real time.
        // Allow a small margin for runners with coarse timers or scheduling jitter.
        assert!(
            elapsed_ts >= 98,
            "Timestamp should advance roughly 100ms, got {}ms",
            elapsed_ts
        );

        // We should still see many millisecond transitions while spinning in a
        // tight loop, but some CI runners (especially Windows) can deschedule
        // the test often enough that we miss a noticeable fraction of them.
        let min_distinct_timestamps = elapsed_ts.saturating_mul(2) / 3;
        assert!(
            distinct_timestamps >= min_distinct_timestamps,
            "Should see frequent updates, got {} distinct timestamps over {}ms",
            distinct_timestamps,
            elapsed_ts
        );
    }

    #[test]
    fn test_record_time_sample_ignores_older_millisecond_samples() {
        let mut state = ThreadState::new();

        assert!(state.record_time_sample(
            clock::TimestampSample {
                ms: 1_000,
                nanos_within_ms: 800_000,
            },
            true
        ));
        assert!(!state.record_time_sample(
            clock::TimestampSample {
                ms: 999,
                nanos_within_ms: 100_000,
            },
            true
        ));

        assert_eq!(state.last_ms, 1_000);
        assert_eq!(state.last_nanos_within_ms, 800_000);
        assert_eq!(state.last_sampled_nanos_within_ms, 800_000);
    }

    #[test]
    fn test_record_time_sample_keeps_same_millisecond_fraction_monotonic() {
        let mut state = ThreadState::new();

        assert!(state.record_time_sample(
            clock::TimestampSample {
                ms: 1_000,
                nanos_within_ms: 400_000,
            },
            true
        ));
        assert!(!state.record_time_sample(
            clock::TimestampSample {
                ms: 1_000,
                nanos_within_ms: 350_000,
            },
            false
        ));
        assert!(!state.record_time_sample(
            clock::TimestampSample {
                ms: 1_000,
                nanos_within_ms: 450_000,
            },
            true
        ));

        assert_eq!(state.current_timestamp_sample().nanos_within_ms, 450_000);
        assert_eq!(state.last_sampled_nanos_within_ms, 450_000);
    }

    #[test]
    fn test_counter_reset() {
        let mut state = ThreadState::new();

        assert!(
            state.refresh_time(),
            "A fresh thread state should accept the first timestamp sample"
        );
        assert_eq!(
            state.counter & !COUNTER_SEED_MASK,
            0,
            "Seeded counter should start in the low 12 bits"
        );
    }

    #[test]
    fn test_counter_rollover_resets_cached_sub_ms_state() {
        let mut state = ThreadState::new();
        assert!(state.refresh_time());
        let previous_ms = state.last_ms;
        state.last_nanos_within_ms = 900_000;
        state.last_sampled_nanos_within_ms = 900_000;
        state.counter = COUNTER_MAX;

        let (timestamp, _) = state.get_time_and_counter();

        assert_eq!(timestamp, previous_ms + 1);
        assert_eq!(state.current_timestamp_sample().nanos_within_ms, 0);
        assert_eq!(state.last_sampled_nanos_within_ms, 0);
    }
}
