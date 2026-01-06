use std::hint::black_box;

use arithmetics::{
    implementations::Element,
    secret_sharing::shamir::{get_lagrange_factors, lagrange, shamir, Parameters},
};
use criterion::{criterion_group, criterion_main, Criterion};

#[cfg(feature = "rug")]
const IMPL: &str = "rug";
#[cfg(feature = "malachite")]
const IMPL: &str = "malachite";
#[cfg(feature = "bigint")]
const IMPL: &str = "bigint";

fn bench_shamir(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("shamir/{}", IMPL));

    let secret = vec![0u8; 1000];

    let mut params = Parameters::new(5, 8).unwrap();

    group.bench_function("generate", |b| {
        b.iter(|| {
            black_box(shamir::<Element>(
                black_box(&secret),
                black_box(&mut params),
            ));
        });
    });

    group.finish();
}

fn bench_lagrange(c: &mut Criterion) {
    let mut group = c.benchmark_group(format!("lagrange/{}", IMPL));

    let secret = vec![42u8; 32];

    let mut params = Parameters::new(5, 8).unwrap();

    // Generate shares once
    let shares = shamir::<Element>(&secret, &mut params);

    let xs: Vec<i32> = (1..=params.threshold()).map(|i| i as i32).collect();

    let factors = get_lagrange_factors::<Element>(&xs).unwrap();

    let ys = &shares[..params.threshold()];

    group.bench_function("reconstruct", |b| {
        b.iter(|| {
            black_box(lagrange::<Element>(black_box(&factors), black_box(ys)).unwrap());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_shamir, bench_lagrange);
criterion_main!(benches);
