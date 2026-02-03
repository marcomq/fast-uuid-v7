use criterion::{criterion_group, criterion_main, Criterion};
use fast_uuid_v7::{gen_id_str, gen_id_string, gen_id_u128};
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
    benchmark_uuid_now_v7,
    benchmark_uuid_now_v7_str
);
criterion_main!(benches);
