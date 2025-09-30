mod common;

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use map_scatter::prelude::{pick_highest_probability, pick_weighted_random, FieldGraphSpec, Kind};
use map_scatter::scatter::evaluator::KindEvaluation;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn make_type_evaluations(count: usize, allowed_ratio: f32, seed: u64) -> Vec<KindEvaluation> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut evals = Vec::with_capacity(count);

    for i in 0..count {
        let allowed = rng.random::<f32>() < allowed_ratio;

        let w = 0.01 + rng.random::<f32>() * 0.99;
        let id = format!("S{}", i);

        evals.push(KindEvaluation {
            kind: Kind::new(id, FieldGraphSpec::default()),
            allowed,
            weight: w,
        });
    }

    evals
}

fn selection_weighted_random_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("selection/weighted_random");

    for &n in &[8usize, 64, 256, 1024, 4096] {
        let evals = make_type_evaluations(n, 0.75, 0xC0FFEE);
        group.throughput(common::elements_throughput(n));

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            let mut rng = StdRng::seed_from_u64(0xDEADBEEF);

            b.iter(|| {
                let sel = pick_weighted_random(&evals, &mut rng);
                black_box(sel);
            });
        });
    }

    for &n in &[256usize, 2048] {
        let mut evals = make_type_evaluations(n, 1.0, 0xFACEFEED);

        for (i, e) in evals.iter_mut().enumerate() {
            e.allowed = true;
            e.weight = 0.25 + ((i % 7) as f32) / 7.0;
        }
        group.throughput(common::elements_throughput(n));

        group.bench_with_input(BenchmarkId::new("all_allowed", n), &n, |b, _| {
            let mut rng = StdRng::seed_from_u64(0xBADC0DE);
            b.iter(|| {
                let sel = pick_weighted_random(&evals, &mut rng);
                black_box(sel);
            });
        });
    }

    for &n in &[256usize, 2048] {
        let evals = make_type_evaluations(n, 0.0, 0x0BADF00D);
        group.throughput(common::elements_throughput(n));

        group.bench_with_input(BenchmarkId::new("none_allowed", n), &n, |b, _| {
            let mut rng = StdRng::seed_from_u64(0xFEED);
            b.iter(|| {
                let sel = pick_weighted_random(&evals, &mut rng);
                black_box(sel);
            });
        });
    }

    group.finish();
}

fn selection_highest_probability_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("selection/highest_probability");

    for &n in &[8usize, 64, 256, 1024, 4096, 16384] {
        let mut evals = make_type_evaluations(n, 1.0, 0x12345678);

        for (i, e) in evals.iter_mut().enumerate() {
            e.allowed = true;
            e.weight = 0.1 + ((i % 97) as f32) / 97.0;
        }
        evals[n - 1].weight = 10.0;

        group.throughput(common::elements_throughput(n));

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let sel = pick_highest_probability(&evals);
                black_box(sel);
            });
        });
    }

    for &n in &[1024usize, 4096] {
        let evals = make_type_evaluations(n, 0.1, 0x87654321);
        group.throughput(common::elements_throughput(n));

        group.bench_with_input(BenchmarkId::new("ten_percent_allowed", n), &n, |b, _| {
            b.iter(|| {
                let sel = pick_highest_probability(&evals);
                black_box(sel);
            });
        });
    }

    group.finish();
}

fn selection_setup_overhead_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("selection/setup_overhead");
    let n = 4096usize;

    {
        let evals = make_type_evaluations(n, 0.75, 0xCAFEBABE);
        group.throughput(common::elements_throughput(n));
        group.bench_function("prebuilt/weighted_random", |b| {
            let mut rng = StdRng::seed_from_u64(0x0);
            b.iter(|| {
                let sel = pick_weighted_random(&evals, &mut rng);
                black_box(sel);
            });
        });
    }

    group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &_n| {
        b.iter_batched(
            || make_type_evaluations(n, 0.75, 0xCAFEBABE),
            |evals| {
                let mut rng = StdRng::seed_from_u64(0x0);
                let sel = pick_weighted_random(&evals, &mut rng);
                black_box(sel);
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = common::default_criterion();
    targets = selection_weighted_random_benches,
              selection_highest_probability_benches,
              selection_setup_overhead_benches
}
criterion_main!(benches);
