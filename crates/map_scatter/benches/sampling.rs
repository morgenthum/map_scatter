mod common;

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::Vec2;
use map_scatter::sampling::jitter_grid::JitterGridSampling;
use map_scatter::sampling::poisson_disk::PoissonDiskSampling;
use map_scatter::sampling::PositionSampling;
use rand::rngs::StdRng;
use rand::SeedableRng;

const JITTER_LEVELS: [f32; 3] = [0.0, 0.5, 0.9];
const RADII: [f32; 6] = [64.0, 32.0, 16.0, 8.0, 4.0, 2.0];

fn sampling_jitter_grid_benches(c: &mut Criterion) {
    let extent = Vec2::new(1024.0, 1024.0);

    for &jitter in &JITTER_LEVELS {
        let mut group = c.benchmark_group(format!("sampling/jitter_grid/jitter_{jitter:.2}"));

        for &radius in &RADII {
            let strategy = JitterGridSampling::new(jitter, radius);
            let mut rng_est = StdRng::seed_from_u64(
                0xA11CE_u64 ^ (radius as u64) ^ ((jitter.to_bits() as u64) << 1) ^ 0xE57,
            );
            let expected = strategy.generate(extent.into(), &mut rng_est).len();
            group.throughput(common::elements_throughput(expected));

            let mut rng = StdRng::seed_from_u64(
                0xA11CE_u64 ^ (radius as u64) ^ ((jitter.to_bits() as u64) << 1),
            );

            group.bench_with_input(BenchmarkId::from_parameter(radius), &radius, |b, _| {
                b.iter(|| {
                    let pts = strategy.generate(extent.into(), &mut rng);
                    black_box(pts.len());
                });
            });
        }

        group.finish();
    }
}

fn sampling_poisson_benches(c: &mut Criterion) {
    let extent = Vec2::new(1024.0, 1024.0);

    let mut group = c.benchmark_group("sampling/poisson_disk");

    for &radius in &RADII {
        let strat_est = PoissonDiskSampling { radius };
        let mut rng_est = StdRng::seed_from_u64(0xBEEFu64 ^ (radius as u64));
        let expected = strat_est.generate(extent.into(), &mut rng_est).len();
        group.throughput(common::elements_throughput(expected));

        let strat = PoissonDiskSampling { radius };
        let mut rng = StdRng::seed_from_u64(0xC0FFEEu64 ^ (radius as u64));

        group.bench_with_input(BenchmarkId::from_parameter(radius), &radius, |b, _| {
            b.iter(|| {
                let pts = strat.generate(extent.into(), &mut rng);
                black_box(pts.len());
            });
        });
    }

    group.finish();
}
criterion_group! {
    name = benches;
    config = common::default_criterion();
    targets = sampling_jitter_grid_benches, sampling_poisson_benches
}
criterion_main!(benches);
