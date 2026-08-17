//  fast-uuid-v7
//  © Copyright 2026, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/fast-uuid-v7

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::{clock, format_uuid, uuid_v7_from_parts, UuidString, COUNTER_MAX, COUNTER_SEED_MASK};

/// Generates strictly increasing UUID v7 values from its own, owned state.
///
/// Unlike the free `gen_id_*` functions, this generator does not touch the
/// thread-local fast path, so using it has no effect on general UUID v7
/// generation. Every `next_id` call on the same instance returns a value
/// strictly greater than the previous one, in both numeric and lexicographic
/// (formatted string) order.
///
/// This makes it suitable for assigning ids to rows read sequentially from a
/// CSV / JSONL file: the ids preserve the input order when the resulting
/// objects are sorted by key, e.g. in S3.
///
/// As prescribed by RFC 9562 §6.2, the 18-bit counter is randomly seeded at
/// each new millisecond; seeding only its low 12 bits leaves at least 258,049
/// ids per millisecond. Beyond that the timestamp is advanced by 1ms to keep
/// the ordering guarantee. 56 bits stay random and the per-millisecond seed
/// makes concurrent instances diverge, so ids from different generator
/// instances remain collision-safe.
///
/// A tight generation loop can outrun that per-millisecond budget on modern
/// hardware — `next_id` costs roughly 3ns, i.e. ~320k ids/ms — which pushes the
/// timestamp ahead of the wall clock. It catches up again as soon as generation
/// pauses, and any real per-row work keeps the rate well below the budget.
///
/// The guarantee is per instance — it is not shared across instances, threads
/// or processes.
///
/// This is not random enough for cryptography!
///
/// # Example
/// ```
/// use fast_uuid_v7::SequentialGenerator;
///
/// let mut gen = SequentialGenerator::new();
/// let mut previous = 0u128;
/// for _row in 0..1000 {
///     let id = gen.next_id();
///     assert!(id > previous);
///     previous = id;
/// }
/// ```
///
/// The generator is also an infinite [`Iterator`], which reads well when ids
/// are zipped onto rows:
/// ```
/// use fast_uuid_v7::SequentialGenerator;
///
/// let rows = ["first", "second", "third"];
/// let mut gen = SequentialGenerator::new();
/// let ided: Vec<(u128, &str)> = gen.by_ref().zip(rows).collect();
/// assert!(ided[0].0 < ided[1].0);
/// ```
pub struct SequentialGenerator {
    rng: SmallRng,
    clock: clock::Clock,
    last_ms: u64,
    counter: u32,
}

impl SequentialGenerator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            rng: SmallRng::from_rng(&mut rand::rng()),
            clock: clock::Clock::new(),
            last_ms: 0,
            counter: 0,
        }
    }

    /// RFC 9562 §6.2 seeds the counter at each new timestamp tick. Randomizing
    /// only the low 12 bits keeps almost all per-millisecond headroom while
    /// making concurrent instances diverge instead of running in lockstep.
    #[inline(always)]
    fn seed_counter(&mut self) -> u32 {
        self.rng.next_u32() & COUNTER_SEED_MASK
    }

    /// Returns the next id, strictly greater than the previously returned one.
    #[inline]
    #[must_use]
    pub fn next_id(&mut self) -> u128 {
        let advanced = if self.last_ms == 0 || self.clock.should_refresh() {
            let sample = self.clock.refresh_timestamp();
            // A backwards-running clock is ignored on purpose: `last_ms` must
            // never decrease, otherwise the ordering guarantee would break.
            if sample.ms > self.last_ms {
                self.last_ms = sample.ms;
                self.counter = self.seed_counter();
                true
            } else {
                false
            }
        } else {
            false
        };

        if !advanced {
            if self.counter >= COUNTER_MAX {
                self.last_ms += 1;
                self.counter = self.seed_counter();
            } else {
                self.counter += 1;
            }
            // The timestamp only ever grows, and `uuid_v7_from_parts` shifts it
            // by 80 bits, so anything wider than 48 bits would silently
            // truncate and break the ordering guarantee.
            debug_assert!(self.last_ms < (1 << 48), "timestamp exceeded 48 bits");
        }

        // 18 bit counter: 12 in rand_a, 6 in the high bits of rand_b, so that
        // the counter stays more significant than the random part.
        let rand_a = ((self.counter >> 6) & 0x0FFF) as u16;
        let rand_b_high = (self.counter & 0x3F) as u64;
        let rand_b_low = self.rng.next_u64() & 0x00FF_FFFF_FFFF_FFFF;

        uuid_v7_from_parts(self.last_ms, rand_a, (rand_b_high << 56) | rand_b_low)
    }

    /// Same as [`SequentialGenerator::next_id`], formatted as a UUID string.
    #[inline]
    #[must_use]
    pub fn next_id_str(&mut self) -> UuidString {
        format_uuid(self.next_id())
    }

    /// Same as [`SequentialGenerator::next_id_str`], but heap-allocated.
    #[inline]
    #[must_use]
    pub fn next_id_string(&mut self) -> String {
        self.next_id_str().to_string()
    }
}

impl Default for SequentialGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Yields ids forever; the sequence is the same one [`SequentialGenerator::next_id`]
/// produces, so it never returns `None`.
impl Iterator for SequentialGenerator {
    type Item = u128;

    #[inline]
    fn next(&mut self) -> Option<u128> {
        Some(self.next_id())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_strictly_increasing() {
        let mut gen = SequentialGenerator::new();
        let mut previous = 0u128;
        for _ in 0..1_000_000 {
            let id = gen.next_id();
            assert!(id > previous);
            previous = id;
        }
    }

    #[test]
    fn strings_sort_like_the_ids() {
        let mut gen = SequentialGenerator::new();
        let ids: Vec<_> = (0..10_000).map(|_| gen.next_id_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn counter_exhaustion_advances_timestamp() {
        let mut gen = SequentialGenerator::new();
        let first = gen.next_id();
        // Far-future timestamp so a clock refresh cannot advance it and mask
        // the rollover path we want to exercise.
        gen.last_ms = 0xF000_0000_0000;
        gen.counter = COUNTER_MAX;
        let ms_before = gen.last_ms;
        let second = gen.next_id();
        assert_eq!(gen.last_ms, ms_before + 1);
        assert!(gen.counter <= COUNTER_SEED_MASK);
        assert!(second > first);
    }

    #[test]
    fn counter_is_reseeded_on_a_new_millisecond() {
        let mut gen = SequentialGenerator::new();
        let first = gen.next_id();
        let ms_before = gen.last_ms;
        // Push the counter beyond any value a reseed could produce, so the
        // assertion below can only pass if the new millisecond reseeded it.
        gen.counter = COUNTER_SEED_MASK + 1000;
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = gen.next_id();
        assert!(gen.last_ms > ms_before);
        assert!(gen.counter <= COUNTER_SEED_MASK);
        assert!(second > first);
    }

    #[test]
    fn instances_do_not_share_one_counter_sequence() {
        let counters: Vec<u32> = (0..8)
            .map(|_| {
                let mut gen = SequentialGenerator::new();
                let _ = gen.next_id();
                gen.counter
            })
            .collect();
        assert!(counters.iter().all(|c| *c <= COUNTER_SEED_MASK));
        // Seeds are 12 bit, so eight identical ones has probability 4096^-7.
        assert!(counters.iter().any(|c| *c != counters[0]));
    }

    #[test]
    fn version_and_variant_survive_counter_rollover() {
        let mut gen = SequentialGenerator::new();
        // Far-future timestamp so clock refreshes cannot advance it and mask
        // the rollover path; re-exhausting the counter forces it every round.
        gen.last_ms = 0xF000_0000_0000;
        let mut previous = 0u128;
        for _ in 0..1000 {
            gen.counter = COUNTER_MAX;
            let id = gen.next_id();
            assert_eq!((id >> 76) & 0xF, 7);
            assert_eq!((id >> 62) & 0x3, 0b10);
            assert!(id > previous);
            previous = id;
        }
    }

    #[test]
    fn iterator_yields_the_same_increasing_sequence() {
        let mut gen = SequentialGenerator::new();
        let ids: Vec<u128> = gen.by_ref().take(10_000).collect();
        assert_eq!(ids.len(), 10_000);
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(gen.next_id() > ids[ids.len() - 1]);
    }

    #[test]
    fn next_id_string_matches_next_id_str() {
        let mut gen = SequentialGenerator::new();
        let owned = gen.next_id_string();
        assert_eq!(owned.len(), 36);
        assert!(owned.as_str() < gen.next_id_str().as_str());
    }

    #[test]
    fn backwards_clock_does_not_break_ordering() {
        let mut gen = SequentialGenerator::new();
        let first = gen.next_id();
        // Simulate a clock that jumped forward once and then back: every later
        // wall-clock sample is now in the past relative to `last_ms`.
        gen.last_ms += 60_000;
        let mut previous = gen.next_id();
        assert!(previous > first);
        for _ in 0..10_000 {
            let id = gen.next_id();
            assert!(id > previous);
            previous = id;
        }
    }

    #[test]
    fn timestamp_matches_wall_clock() {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let id = SequentialGenerator::new().next_id();
        let timestamp = (id >> 80) as u64;
        assert!(timestamp.abs_diff(now_ms) < 1000);
    }

    #[test]
    fn instances_are_independent() {
        let (mut first, mut second) = (SequentialGenerator::new(), SequentialGenerator::new());
        assert_ne!(first.next_id(), second.next_id());
    }

    #[test]
    fn string_and_u128_orderings_agree() {
        let mut gen = SequentialGenerator::new();
        let ids: Vec<u128> = (0..10_000).map(|_| gen.next_id()).collect();
        for pair in ids.windows(2) {
            assert!(format_uuid(pair[0]).as_str() < format_uuid(pair[1]).as_str());
        }
    }

    #[test]
    fn version_and_variant_are_preserved() {
        let mut gen = SequentialGenerator::new();
        for _ in 0..1000 {
            let id = gen.next_id();
            assert_eq!((id >> 76) & 0xF, 7);
            assert_eq!((id >> 62) & 0x3, 0b10);
        }
    }
}
