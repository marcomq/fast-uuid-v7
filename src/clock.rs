const NANOS_PER_MS: u64 = 1_000_000;
const MAX_REFRESH_INTERVAL_NANOS: u64 = NANOS_PER_MS / 2;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use counter_clock::CounterClock as Backend;
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
use system_clock::SystemClock as Backend;

#[derive(Clone, Copy)]
pub(crate) struct TimestampSample {
    pub(crate) ms: u64,
    pub(crate) nanos_within_ms: u32,
}

impl TimestampSample {
    #[inline(always)]
    pub(crate) fn sub_ms_fraction(&self, bits: u8) -> u16 {
        debug_assert!(bits <= 12);

        if bits == 0 {
            return 0;
        }

        let nanos_within_ms = self.nanos_within_ms.min(999_999);
        let slots = 1u32 << bits;
        ((nanos_within_ms * slots) / 1_000_000) as u16
    }
}

pub(crate) struct Clock {
    backend: Backend,
}

impl Clock {
    pub(crate) fn new() -> Self {
        Self {
            backend: Backend::new(),
        }
    }

    #[inline(always)]
    pub(crate) fn should_refresh(&self) -> bool {
        self.backend.should_refresh()
    }

    #[inline(always)]
    pub(crate) fn record_sample(&mut self, nanos_within_ms: u32, sampled_at: u64) {
        self.backend.record_sample(nanos_within_ms, sampled_at);
    }

    #[inline(always)]
    pub(crate) fn estimate_nanos_within_ms(&self, sampled_nanos_within_ms: u32) -> Option<u32> {
        self.backend
            .estimate_nanos_within_ms(sampled_nanos_within_ms)
    }

    #[inline(always)]
    pub(crate) fn refresh_timestamp(&mut self) -> TimestampSample {
        let (sample, sampled_at) = system_time_sample_with_counter();
        self.record_sample(sample.nanos_within_ms, sampled_at);
        sample
    }

    #[inline(always)]
    pub(crate) fn estimated_timestamp(
        &self,
        last_ms: u64,
        sampled_nanos_within_ms: u32,
    ) -> TimestampSample {
        TimestampSample {
            ms: last_ms,
            nanos_within_ms: self
                .estimate_nanos_within_ms(sampled_nanos_within_ms)
                .unwrap_or(sampled_nanos_within_ms),
        }
    }
}

#[inline]
fn system_time_sample() -> TimestampSample {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    TimestampSample {
        ms: duration.as_millis() as u64,
        nanos_within_ms: duration.subsec_nanos() % 1_000_000,
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[inline(always)]
fn system_time_sample_with_counter() -> (TimestampSample, u64) {
    let before = read_counter();
    let sample = system_time_sample();
    let after = read_counter();
    let sampled_at = before.wrapping_add(after.wrapping_sub(before) / 2);
    (sample, sampled_at)
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline(always)]
fn system_time_sample_with_counter() -> (TimestampSample, u64) {
    (system_time_sample(), 0)
}

#[inline(always)]
fn nanos_until_next_ms(nanos_within_ms: u32) -> u64 {
    NANOS_PER_MS - (u64::from(nanos_within_ms) % NANOS_PER_MS)
}

#[inline(always)]
fn nanos_until_next_refresh(nanos_within_ms: u32) -> u64 {
    nanos_until_next_ms(nanos_within_ms).min(MAX_REFRESH_INTERVAL_NANOS)
}

#[inline(always)]
#[cfg(any(test, any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(super) fn estimate_nanos_within_ms_from_ticks(
    ticks_per_ms: u64,
    sampled_nanos_within_ms: u32,
    elapsed_ticks: u64,
) -> u32 {
    let elapsed_nanos = elapsed_ticks.saturating_mul(1_000_000) / ticks_per_ms.max(1);
    let estimated = u64::from(sampled_nanos_within_ms).saturating_add(elapsed_nanos);
    estimated.min(999_999) as u32
}

#[inline(always)]
#[cfg(any(test, any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(super) fn ticks_until_next_refresh(ticks_per_ms: u64, nanos_within_ms: u32) -> u64 {
    let ticks = ticks_per_ms
        .saturating_mul(nanos_until_next_refresh(nanos_within_ms))
        .saturating_add(NANOS_PER_MS - 1)
        / NANOS_PER_MS;
    ticks.max(1)
}

#[inline(always)]
#[cfg(any(test, any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(super) fn deadline_reached(current: u64, deadline: u64) -> bool {
    deadline == 0 || current.wrapping_sub(deadline) < (1u64 << 63)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn read_counter() -> u64 {
    // SAFETY: _rdtsc is available on x86_64.
    unsafe { std::arch::x86_64::_rdtsc() }
}

#[cfg(target_arch = "x86_64")]
fn counter_ticks_per_ms() -> u64 {
    // Default to 2GHz (2000 MHz) if detection fails.
    let mut base_mhz = 2000;

    let max_leaf = std::arch::x86_64::__get_cpuid_max(0).0;
    if max_leaf >= 0x16 {
        let res = std::arch::x86_64::__cpuid(0x16);
        if res.eax > 0 {
            base_mhz = res.eax as u64;
        }
    }

    base_mhz * 1000
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn read_counter() -> u64 {
    let current_tsc: u64;
    // SAFETY: reading cntvct_el0 is safe in userspace.
    unsafe {
        std::arch::asm!(
            "mrs {}, cntvct_el0",
            out(reg) current_tsc,
            options(nomem, nostack, preserves_flags)
        );
    }
    current_tsc
}

#[cfg(target_arch = "aarch64")]
fn counter_ticks_per_ms() -> u64 {
    let freq: u64;
    // SAFETY: reading cntfrq_el0 is safe in userspace on Linux/macOS.
    unsafe {
        std::arch::asm!(
            "mrs {}, cntfrq_el0",
            out(reg) freq,
            options(nomem, nostack, preserves_flags)
        );
    }

    (freq / 1000).max(1)
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
mod counter_clock {
    use super::{
        deadline_reached, estimate_nanos_within_ms_from_ticks, read_counter,
        ticks_until_next_refresh,
    };

    pub(super) struct CounterClock {
        ticks_per_ms: u64,
        sampled_at: u64,
        next_deadline: u64,
    }

    impl CounterClock {
        pub(super) fn new() -> Self {
            Self {
                ticks_per_ms: super::counter_ticks_per_ms(),
                sampled_at: 0,
                next_deadline: 0,
            }
        }

        #[inline(always)]
        pub(super) fn should_refresh(&self) -> bool {
            deadline_reached(read_counter(), self.next_deadline)
        }

        #[inline(always)]
        pub(super) fn record_sample(&mut self, nanos_within_ms: u32, sampled_at: u64) {
            let ticks_until_next_refresh =
                ticks_until_next_refresh(self.ticks_per_ms, nanos_within_ms);
            self.sampled_at = sampled_at;
            self.next_deadline = sampled_at.wrapping_add(ticks_until_next_refresh);
        }

        #[inline(always)]
        pub(super) fn estimate_nanos_within_ms(&self, sampled_nanos_within_ms: u32) -> Option<u32> {
            if self.next_deadline == 0 {
                return None;
            }

            let elapsed_ticks = read_counter().wrapping_sub(self.sampled_at);
            Some(estimate_nanos_within_ms_from_ticks(
                self.ticks_per_ms,
                sampled_nanos_within_ms,
                elapsed_ticks,
            ))
        }
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
mod system_clock {
    pub(super) struct SystemClock;

    impl SystemClock {
        pub(super) fn new() -> Self {
            Self
        }

        #[inline(always)]
        pub(super) fn should_refresh(&self) -> bool {
            true
        }

        #[inline(always)]
        pub(super) fn record_sample(&mut self, _nanos_within_ms: u32, _sampled_at: u64) {}

        #[inline(always)]
        pub(super) fn estimate_nanos_within_ms(
            &self,
            _sampled_nanos_within_ms: u32,
        ) -> Option<u32> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_nanos_within_ms_from_ticks_interpolates_fraction() {
        assert_eq!(
            estimate_nanos_within_ms_from_ticks(1_000, 250_000, 500),
            750_000
        );
    }

    #[test]
    fn test_estimate_nanos_within_ms_from_ticks_clamps_to_end_of_ms() {
        assert_eq!(
            estimate_nanos_within_ms_from_ticks(1_000, 900_000, 200),
            999_999
        );
    }

    #[test]
    fn test_sub_ms_fraction_clamps_exact_millisecond_boundary() {
        let sample = TimestampSample {
            ms: 0,
            nanos_within_ms: 1_000_000,
        };

        assert_eq!(sample.sub_ms_fraction(4), 15);
        assert_eq!(sample.sub_ms_fraction(8), 255);
        assert_eq!(sample.sub_ms_fraction(12), 4095);
    }
}
