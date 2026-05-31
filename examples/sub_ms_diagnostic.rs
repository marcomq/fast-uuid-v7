use fast_uuid_v7::{gen_id_with_sub_ms_12, gen_id_with_sub_ms_4, gen_id_with_sub_ms_8};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_SAMPLES: usize = 20_000;

fn main() {
    let samples = std::env::args()
        .nth(1)
        .map(|arg| {
            arg.parse::<usize>()
                .expect("sample count must be a positive integer")
        })
        .unwrap_or(DEFAULT_SAMPLES);

    println!("Sub-ms diagnostic using SystemTime before/after windows");
    println!("Samples per variant: {samples}");
    println!();

    run_variant("gen_id_with_sub_ms_4", 4, gen_id_with_sub_ms_4, samples);
    run_variant("gen_id_with_sub_ms_8", 8, gen_id_with_sub_ms_8, samples);
    run_variant("gen_id_with_sub_ms_12", 12, gen_id_with_sub_ms_12, samples);
}

fn run_variant(name: &str, bits: u8, gen_id: fn() -> u128, samples: usize) {
    let mut interval_errors_ns = Vec::with_capacity(samples);
    let mut midpoint_errors_ns = Vec::with_capacity(samples);
    let mut signed_midpoint_errors_ns = Vec::with_capacity(samples);
    let mut call_windows_ns = Vec::with_capacity(samples);
    let mut overlap_count = 0usize;
    let mut lagging_count = 0usize;
    let mut leading_count = 0usize;

    for _ in 0..samples {
        let before = unix_time_ns();
        let id = gen_id();
        let after = unix_time_ns();

        let (id_start_ns, id_end_ns) = decode_sub_ms_window_ns(id, bits);
        let interval_error_ns = interval_gap_ns(id_start_ns, id_end_ns, before, after);
        let observed_mid_ns = before + (after.saturating_sub(before) / 2);
        let id_mid_ns = id_start_ns + ((id_end_ns.saturating_sub(id_start_ns)) / 2);
        let midpoint_error_ns = id_mid_ns.abs_diff(observed_mid_ns);
        let signed_midpoint_error_ns = id_mid_ns as i128 - observed_mid_ns as i128;

        if interval_error_ns == 0 {
            overlap_count += 1;
        }
        if signed_midpoint_error_ns < 0 {
            lagging_count += 1;
        } else if signed_midpoint_error_ns > 0 {
            leading_count += 1;
        }

        interval_errors_ns.push(interval_error_ns);
        midpoint_errors_ns.push(midpoint_error_ns);
        signed_midpoint_errors_ns.push(signed_midpoint_error_ns);
        call_windows_ns.push(after.saturating_sub(before));
    }

    interval_errors_ns.sort_unstable();
    midpoint_errors_ns.sort_unstable();
    signed_midpoint_errors_ns.sort_unstable();
    call_windows_ns.sort_unstable();

    println!("{name} ({bits} bits)");
    print_stats("call window", &call_windows_ns);
    print_stats("interval gap", &interval_errors_ns);
    print_stats("midpoint error", &midpoint_errors_ns);
    print_signed_stats("signed drift", &signed_midpoint_errors_ns);
    println!(
        "  overlap rate : {:>6.2}%\n  lagging rate : {:>6.2}%\n  leading rate : {:>6.2}%",
        percentage(overlap_count, samples),
        percentage(lagging_count, samples),
        percentage(leading_count, samples),
    );
    println!(
        "  <=100us gap  : {:>6.2}%\n  <=250us gap  : {:>6.2}%\n  <=500us gap  : {:>6.2}%\n  <=1ms gap    : {:>6.2}%",
        percentage_within(&interval_errors_ns, 100_000),
        percentage_within(&interval_errors_ns, 250_000),
        percentage_within(&interval_errors_ns, 500_000),
        percentage_within(&interval_errors_ns, 1_000_000),
    );
    println!();
}

fn unix_time_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos()
}

fn decode_sub_ms_window_ns(id: u128, bits: u8) -> (u128, u128) {
    let ms = id >> 80;
    let rand_a = (id >> 64) & 0x0FFF;
    let fraction = rand_a >> (12 - bits);
    let slots = 1u128 << bits;
    let base_ns = ms * 1_000_000;
    let start_ns = base_ns + (fraction * 1_000_000) / slots;
    let end_ns = base_ns + (((fraction + 1) * 1_000_000).saturating_sub(1)) / slots;
    (start_ns, end_ns)
}

fn interval_gap_ns(a_start: u128, a_end: u128, b_start: u128, b_end: u128) -> u128 {
    if a_end < b_start {
        b_start - a_end
    } else if b_end < a_start {
        a_start - b_end
    } else {
        0
    }
}

fn print_stats(label: &str, values: &[u128]) {
    println!(
        "  {label:<14} p50={:>8}us  p90={:>8}us  p99={:>8}us  max={:>8}us",
        ns_to_us(percentile(values, 50)),
        ns_to_us(percentile(values, 90)),
        ns_to_us(percentile(values, 99)),
        ns_to_us(*values.last().unwrap_or(&0)),
    );
}

fn print_signed_stats(label: &str, values: &[i128]) {
    println!(
        "  {label:<14} p10={:>8}us  p50={:>8}us  p90={:>8}us",
        signed_ns_to_us(percentile_signed(values, 10)),
        signed_ns_to_us(percentile_signed(values, 50)),
        signed_ns_to_us(percentile_signed(values, 90)),
    );
}

fn percentile(sorted: &[u128], percentile: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }

    let rank = ((sorted.len() - 1) * percentile) / 100;
    sorted[rank]
}

fn ns_to_us(value: u128) -> u128 {
    value / 1_000
}

fn signed_ns_to_us(value: i128) -> i128 {
    value / 1_000
}

fn percentile_signed(sorted: &[i128], percentile: usize) -> i128 {
    if sorted.is_empty() {
        return 0;
    }

    let rank = ((sorted.len() - 1) * percentile) / 100;
    sorted[rank]
}

fn percentage_within(sorted: &[u128], limit_ns: u128) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let count = sorted.partition_point(|value| *value <= limit_ns);
    percentage(count, sorted.len())
}

fn percentage(count: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }

    (count as f64 * 100.0) / total as f64
}
