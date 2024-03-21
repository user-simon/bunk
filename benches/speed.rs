use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use bunk::*;
use rand::{rngs::SmallRng, RngCore, SeedableRng};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = SmallRng::from_entropy();

    c.bench_function("round-trip 32", |b| {
        let setup = || {
            let mut data = [0; 32];
            rng.fill_bytes(&mut data);
            data
        };
        let routine = |bytes| {
            let encoded = encode(bytes);
            let decoded = decode(&encoded);
            (encoded, decoded)
        };
        b.iter_batched(setup, routine, BatchSize::SmallInput)
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
