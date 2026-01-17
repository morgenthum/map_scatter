mod common;

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use glam::Vec2;
use map_scatter::prelude::{
    run_plan, FieldGraphSpec, FieldProgramCache, FieldSemantics, JitterGridSampling, Layer,
    NodeSpec, Plan, PoissonDiskSampling, PositionSampling, RunConfig, TextureRegistry,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn make_scatter_kind_spec(probability: f32) -> FieldGraphSpec {
    let mut spec = FieldGraphSpec::default();

    spec.add("gate_ok", NodeSpec::constant(1.0));
    spec.set_semantics("gate_ok", FieldSemantics::Gate);

    spec.add("probability", NodeSpec::constant(probability));
    spec.set_semantics("probability", FieldSemantics::Probability);

    spec
}

fn build_plan(
    num_stages: usize,
    layers_per_stage: usize,
    kinds_per_layer: usize,
    base_radius: f32,
    mut make_sampling: impl FnMut(f32) -> Box<dyn PositionSampling>,
) -> Plan {
    let mut plan = Plan::new();

    for s in 0..num_stages {
        for l in 0..layers_per_stage {
            let mut kinds = Vec::with_capacity(kinds_per_layer);
            for t in 0..kinds_per_layer {
                let prob = 0.2 + (t as f32 + 1.0) / (kinds_per_layer as f32 + 1.0) * 0.8;
                let spec = make_scatter_kind_spec(prob);
                let scatter_kind =
                    map_scatter::scatter::Kind::new(format!("sp_s{s}_l{l}_t{t}"), spec);
                kinds.push(scatter_kind);
            }

            let separation = base_radius * (1.0 + (l as f32) * 0.15);
            let sampling = make_sampling(separation);
            let layer = Layer::new(format!("layer_{s}_{l}"), kinds, sampling);
            plan = plan.with_layer(layer);
        }
    }

    plan
}

fn bench_with_sampling(
    c: &mut Criterion,
    bench_name: &str,
    mut make_layer_sampling: impl FnMut(f32) -> Box<dyn PositionSampling>,
) {
    let domain_extent = Vec2::new(1024.0, 1024.0);
    let base_radius = 6.0;

    let plan = build_plan(2, 2, 3, base_radius, |sep| make_layer_sampling(sep));

    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(128.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let textures = TextureRegistry::new();
    let cache = FieldProgramCache::new();

    let mut group = c.benchmark_group(bench_name);
    // Preview a run to set meaningful throughput in "placements per iteration".
    let mut rng_preview = StdRng::seed_from_u64(0xD3ADB33F);
    let preview = run_plan(
        &plan,
        &config,
        &textures,
        &cache,
        &mut rng_preview,
        None,
    );
    let expected = preview.placements.len();
    group.throughput(common::elements_throughput(expected));

    group.bench_function("run_plan", |b| {
        b.iter_batched(
            || StdRng::seed_from_u64(12345),
            |mut rng| {
                let result = run_plan(&plan, &config, &textures, &cache, &mut rng, None);

                black_box(result.positions_evaluated);
                black_box(result.placements.len());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

#[allow(clippy::too_many_arguments)]
fn bench_with_sampling_params(
    c: &mut Criterion,
    bench_name: &str,
    domain_extent: Vec2,
    num_stages: usize,
    layers_per_stage: usize,
    kinds_per_layer: usize,
    base_radius: f32,
    mut make_layer_sampling: impl FnMut(f32) -> Box<dyn PositionSampling>,
) {
    let plan = build_plan(
        num_stages,
        layers_per_stage,
        kinds_per_layer,
        base_radius,
        |sep| make_layer_sampling(sep),
    );

    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(128.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let textures = TextureRegistry::new();
    let cache = FieldProgramCache::new();

    let mut group = c.benchmark_group(bench_name);

    // Preview a run to set meaningful throughput in "placements per iteration".
    let mut rng_preview = StdRng::seed_from_u64(0xFEEDFACE);
    let preview = run_plan(
        &plan,
        &config,
        &textures,
        &cache,
        &mut rng_preview,
        None,
    );
    let expected = preview.placements.len();
    group.throughput(common::elements_throughput(expected));

    group.bench_function("run_plan", |b| {
        b.iter_batched(
            || StdRng::seed_from_u64(12345),
            |mut rng| {
                let result = run_plan(&plan, &config, &textures, &cache, &mut rng, None);
                black_box(result.positions_evaluated);
                black_box(result.placements.len());
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

pub fn scatter_benches(c: &mut Criterion) {
    // Default size, simple plan
    bench_with_sampling(c, "scatter/jitter_grid/1024/simple", |sep| {
        Box::new(JitterGridSampling::new(0.75, sep))
    });

    bench_with_sampling_params(
        c,
        "scatter/jitter_grid/512/simple",
        Vec2::new(512.0, 512.0),
        2,
        2,
        3,
        6.0,
        |sep| Box::new(JitterGridSampling::new(0.75, sep)),
    );

    bench_with_sampling_params(
        c,
        "scatter/jitter_grid/2048/simple",
        Vec2::new(2048.0, 2048.0),
        2,
        2,
        3,
        6.0,
        |sep| Box::new(JitterGridSampling::new(0.75, sep)),
    );

    bench_with_sampling_params(
        c,
        "scatter/jitter_grid/1024/complex",
        Vec2::new(1024.0, 1024.0),
        3,
        4,
        8,
        6.0,
        |sep| Box::new(JitterGridSampling::new(0.75, sep)),
    );

    // Default size, simple plan
    bench_with_sampling(c, "scatter/poisson_disk/1024/simple", |sep| {
        Box::new(PoissonDiskSampling { radius: sep })
    });

    bench_with_sampling_params(
        c,
        "scatter/poisson_disk/512/simple",
        Vec2::new(512.0, 512.0),
        2,
        2,
        3,
        6.0,
        |sep| Box::new(PoissonDiskSampling { radius: sep }),
    );

    bench_with_sampling_params(
        c,
        "scatter/poisson_disk/2048/simple",
        Vec2::new(2048.0, 2048.0),
        2,
        2,
        3,
        6.0,
        |sep| Box::new(PoissonDiskSampling { radius: sep }),
    );

    bench_with_sampling_params(
        c,
        "scatter/poisson_disk/1024/complex",
        Vec2::new(1024.0, 1024.0),
        3,
        4,
        8,
        6.0,
        |sep| Box::new(PoissonDiskSampling { radius: sep }),
    );
}

criterion_group! {
    name = benches;
    config = common::default_criterion();
    targets = scatter_benches
}
criterion_main!(benches);
