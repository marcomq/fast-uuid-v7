const NANOS_PER_MS: u64 = 1_000_000;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use counter_clock::CounterClock as Backend;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
use system_clock::SystemClock as Backend;

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
    pub(crate) fn record_sample(&mut self, nanos_within_ms: u32) {
        self.backend.record_sample(nanos_within_ms);
    }
}

#[inline(always)]
fn nanos_until_next_ms(nanos_within_ms: u32) -> u64 {
    NANOS_PER_MS - (u64::from(nanos_within_ms) % NANOS_PER_MS)
}

#[inline(always)]
#[cfg(any(test, any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(super) fn ticks_until_next_ms(ticks_per_ms: u64, nanos_within_ms: u32) -> u64 {
    let ticks = ticks_per_ms
        .saturating_mul(nanos_until_next_ms(nanos_within_ms))
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

    // SAFETY: cpuid is safe on x86_64.
    unsafe {
        let max_leaf = std::arch::x86_64::__get_cpuid_max(0).0;
        if max_leaf >= 0x16 {
            let res = std::arch::x86_64::__cpuid(0x16);
            if res.eax > 0 {
                base_mhz = res.eax as u64;
            }
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
    use super::{deadline_reached, read_counter, ticks_until_next_ms};

    pub(super) struct CounterClock {
        ticks_per_ms: u64,
        next_deadline: u64,
    }

    impl CounterClock {
        pub(super) fn new() -> Self {
            Self {
                ticks_per_ms: super::counter_ticks_per_ms(),
                next_deadline: 0,
            }
        }

        #[inline(always)]
        pub(super) fn should_refresh(&self) -> bool {
            deadline_reached(read_counter(), self.next_deadline)
        }

        #[inline(always)]
        pub(super) fn record_sample(&mut self, nanos_within_ms: u32) {
            let ticks_until_next_ms = ticks_until_next_ms(self.ticks_per_ms, nanos_within_ms);
            self.next_deadline = read_counter().wrapping_add(ticks_until_next_ms);
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
        pub(super) fn record_sample(&mut self, _nanos_within_ms: u32) {}
    }
}
