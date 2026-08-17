use criterion::{criterion_group, criterion_main, Criterion};
use fast_uuid_v7::{
    format_uuid, format_uuid_hex, gen_id_str, gen_id_string, gen_id_u128, gen_id_with_count,
    gen_id_with_count_str, gen_id_with_sub_ms_8, SequentialGenerator,
};
use std::hint::black_box;
use uuid::Uuid;

fn benchmark_gen_id_u128(c: &mut Criterion) {
    c.bench_function("gen_id_u128", |b| b.iter(|| gen_id_u128()));
}

fn benchmark_gen_id_string(c: &mut Criterion) {
    c.bench_function("gen_id_string", |b| b.iter(|| gen_id_string()));
}

fn benchmark_gen_id_str(c: &mut Criterion) {
    c.bench_function("gen_id_str", |b| b.iter(|| gen_id_str()));
}

fn benchmark_format_uuid(c: &mut Criterion) {
    c.bench_function("format_uuid", |b| {
        b.iter(|| format_uuid(black_box(0x019e_9bb4_c7c3_77a4_8a83_d613_6b3e_4cebu128)))
    });
}

fn benchmark_format_uuid_hex(c: &mut Criterion) {
    c.bench_function("format_uuid_hex", |b| {
        b.iter(|| format_uuid_hex(black_box(0x019e_9bb4_c7c3_77a4_8a83_d613_6b3e_4cebu128)))
    });
}

fn benchmark_gen_id_with_count(c: &mut Criterion) {
    c.bench_function("gen_id_with_count", |b| b.iter(|| gen_id_with_count()));
}

fn benchmark_gen_id_with_count_str(c: &mut Criterion) {
    c.bench_function("gen_id_with_count_str", |b| {
        b.iter(|| gen_id_with_count_str())
    });
}

fn benchmark_gen_id_with_sub_ms_8(c: &mut Criterion) {
    c.bench_function("gen_id_with_sub_ms_8", |b| {
        b.iter(|| gen_id_with_sub_ms_8())
    });
}

fn benchmark_sequential_next_id(c: &mut Criterion) {
    let mut gen = SequentialGenerator::new();
    c.bench_function("sequential_next_id", |b| b.iter(|| gen.next_id()));
}

fn benchmark_sequential_next_id_str(c: &mut Criterion) {
    let mut gen = SequentialGenerator::new();
    c.bench_function("sequential_next_id_str", |b| b.iter(|| gen.next_id_str()));
}

fn benchmark_uuid_now_v7(c: &mut Criterion) {
    c.bench_function("uuid_now_v7", |b| b.iter(|| Uuid::now_v7()));
}

fn benchmark_uuid_now_v7_str(c: &mut Criterion) {
    c.bench_function("uuid_now_v7_str", |b| b.iter(|| Uuid::now_v7().to_string()));
}

criterion_group!(
    benches,
    benchmark_gen_id_u128,
    benchmark_gen_id_string,
    benchmark_gen_id_str,
    benchmark_format_uuid,
    benchmark_format_uuid_hex,
    benchmark_gen_id_with_count,
    benchmark_gen_id_with_count_str,
    benchmark_gen_id_with_sub_ms_8,
    benchmark_sequential_next_id,
    benchmark_sequential_next_id_str,
    benchmark_uuid_now_v7,
    benchmark_uuid_now_v7_str
);
criterion_main!(benches);
