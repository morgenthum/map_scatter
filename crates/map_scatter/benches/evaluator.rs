mod common;

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use glam::Vec2;
use map_scatter::prelude::{
    FieldGraphSpec, FieldProgramCache, FieldSemantics, Kind, NodeSpec, TextureRegistry,
};
use map_scatter::scatter::chunk::chunk_id_and_grid_for_position_centered;
use map_scatter::scatter::evaluator::Evaluator;

const TYPE_COUNTS: [usize; 6] = [1, 4, 16, 64, 128, 256];

fn make_type_spec(probability: f32) -> FieldGraphSpec {
    let mut spec = FieldGraphSpec::default();

    spec.add("gate_ok", NodeSpec::constant(1.0));
    spec.set_semantics("gate_ok", FieldSemantics::Gate);

    spec.add(
        "probability",
        NodeSpec::constant(probability.clamp(0.0, 1.0)),
    );
    spec.set_semantics("probability", FieldSemantics::Probability);

    spec
}

fn base_probability(i: usize, count: usize) -> f32 {
    0.2 + ((i as f32 + 1.0) / (count as f32 + 1.0)) * 0.8
}

fn make_type_spec_complex(probability: f32) -> FieldGraphSpec {
    let mut spec = FieldGraphSpec::default();

    // Gate
    spec.add("gate_ok", NodeSpec::constant(1.0));
    spec.set_semantics("gate_ok", FieldSemantics::Gate);

    // Build a small arithmetic graph to increase runtime work.
    spec.add("p_base", NodeSpec::constant(probability.clamp(0.0, 1.0)));
    spec.add("p_pow", NodeSpec::pow("p_base".into(), 3.0));
    spec.add("p_step", NodeSpec::smoothstep("p_pow".into(), 0.2, 0.8));
    spec.add("p_scale", NodeSpec::scale("p_step".into(), 0.85));
    spec.add("p_inv", NodeSpec::invert("p_scale".into()));
    spec.add(
        "p_mix",
        NodeSpec::mul(vec!["p_scale".into(), "p_inv".into()]),
    );

    spec.add("probability", NodeSpec::clamp("p_mix".into(), 0.0, 1.0));
    spec.set_semantics("probability", FieldSemantics::Probability);

    spec
}

fn make_kinds_complex(count: usize) -> Vec<Kind> {
    (0..count)
        .map(|i| {
            let p = base_probability(i, count);
            Kind::new(format!("type_{i}"), make_type_spec_complex(p))
        })
        .collect()
}

fn make_kinds(count: usize) -> Vec<Kind> {
    (0..count)
        .map(|i| {
            let p = 0.2 + ((i as f32 + 1.0) / (count as f32 + 1.0)) * 0.8;
            Kind::new(format!("type_{i}"), make_type_spec(p))
        })
        .collect()
}

fn generate_grid_positions(extent: Vec2, nx: usize, ny: usize) -> Vec<Vec2> {
    let hw = extent.x * 0.5;
    let hh = extent.y * 0.5;

    let dx = if nx > 0 {
        extent.x / nx as f32
    } else {
        extent.x
    };
    let dy = if ny > 0 {
        extent.y / ny as f32
    } else {
        extent.y
    };

    let mut pts = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            let x = -hw + (i as f32 + 0.5) * dx;
            let y = -hh + (j as f32 + 0.5) * dy;
            pts.push(Vec2::new(x, y));
        }
    }
    pts
}

fn precompute_chunks_for_positions(
    positions: &[Vec2],
    domain_extent: Vec2,
    chunk_extent: f32,
    raster_cell_size: f32,
    grid_halo: usize,
) -> Vec<(
    map_scatter::fieldgraph::ChunkId,
    map_scatter::fieldgraph::ChunkGrid,
)> {
    positions
        .iter()
        .map(|&p| {
            chunk_id_and_grid_for_position_centered(
                p,
                domain_extent,
                chunk_extent,
                raster_cell_size,
                grid_halo,
            )
        })
        .collect()
}

fn evaluator_run_benches(c: &mut Criterion) {
    let domain_extent = Vec2::new(1024.0, 1024.0);
    let chunk_extent = 128.0;
    let raster_cell_size = 1.0;
    let grid_halo = 2;
    let positions = generate_grid_positions(domain_extent, 64, 64);
    let chunk_and_grids = precompute_chunks_for_positions(
        &positions,
        domain_extent,
        chunk_extent,
        raster_cell_size,
        grid_halo,
    );

    let textures = TextureRegistry::new();

    let mut group = c.benchmark_group("evaluator/evaluate_position");
    for &type_count in &TYPE_COUNTS {
        let throughput = positions.len() * type_count;
        group.throughput(common::elements_throughput(throughput));

        {
            let cache = FieldProgramCache::new();
            let kinds = make_kinds(type_count);
            let evaluator = Evaluator::new(&kinds, &cache).expect("compile ok");

            group.bench_with_input(
                BenchmarkId::new("simple", type_count),
                &type_count,
                |b, _| {
                    b.iter(|| {
                        let mut total_result_items = 0usize;

                        for (&pos, (chunk, grid)) in positions.iter().zip(&chunk_and_grids) {
                            let pos = black_box(pos);
                            let res =
                                evaluator.evaluate_position(pos, *chunk, grid, &kinds, &textures);
                            total_result_items += res.len();
                            black_box(total_result_items);
                        }
                    });
                },
            );
        }

        {
            let cache = FieldProgramCache::new();
            let kinds = make_kinds_complex(type_count);
            let evaluator = Evaluator::new(&kinds, &cache).expect("compile ok");

            group.bench_with_input(
                BenchmarkId::new("complex", type_count),
                &type_count,
                |b, _| {
                    b.iter(|| {
                        let mut total_result_items = 0usize;

                        for (&pos, (chunk, grid)) in positions.iter().zip(&chunk_and_grids) {
                            let pos = black_box(pos);
                            let res =
                                evaluator.evaluate_position(pos, *chunk, grid, &kinds, &textures);
                            total_result_items += res.len();
                            black_box(total_result_items);
                        }
                    });
                },
            );
        }
    }

    group.finish();
}

fn evaluator_compile_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluator/compile");

    for &type_count in &TYPE_COUNTS {
        let types_simple = make_kinds(type_count);
        let types_complex = make_kinds_complex(type_count);

        group.throughput(common::elements_throughput(type_count));

        group.bench_with_input(
            BenchmarkId::new("cold/simple", type_count),
            &type_count,
            |b, _| {
                b.iter_batched(
                    FieldProgramCache::new,
                    |cache| {
                        let eval = Evaluator::new(&types_simple, &cache).expect("compile ok");
                        black_box(eval);
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cold/complex", type_count),
            &type_count,
            |b, _| {
                b.iter_batched(
                    FieldProgramCache::new,
                    |cache| {
                        let eval = Evaluator::new(&types_complex, &cache).expect("compile ok");
                        black_box(eval);
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("warm/simple", type_count),
            &type_count,
            |b, _| {
                let cache = FieldProgramCache::new();
                let _ = Evaluator::new(&types_simple, &cache).expect("compile ok");

                b.iter(|| {
                    let eval = Evaluator::new(&types_simple, &cache).expect("compile ok");
                    black_box(eval);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("warm/complex", type_count),
            &type_count,
            |b, _| {
                let cache = FieldProgramCache::new();
                let _ = Evaluator::new(&types_complex, &cache).expect("compile ok");

                b.iter(|| {
                    let eval = Evaluator::new(&types_complex, &cache).expect("compile ok");
                    black_box(eval);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("warm_change/simple", type_count),
            &type_count,
            |b, &_tc| {
                let cache = FieldProgramCache::new();
                let mut tick: u32 = 0;

                b.iter_batched(
                    || {
                        tick = tick.wrapping_add(1);
                        let delta = (tick % 10) as f32 * 1e-6;
                        (0..type_count)
                            .map(|i| {
                                let p = base_probability(i, type_count) + delta;
                                Kind::new(format!("type_{i}"), make_type_spec(p))
                            })
                            .collect::<Vec<_>>()
                    },
                    |types| {
                        let eval = Evaluator::new(&types, &cache).expect("compile ok");
                        black_box(eval);
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("warm_change/complex", type_count),
            &type_count,
            |b, &_tc| {
                let cache = FieldProgramCache::new();
                let mut tick: u32 = 0;

                b.iter_batched(
                    || {
                        tick = tick.wrapping_add(1);
                        let delta = (tick % 10) as f32 * 1e-6;
                        (0..type_count)
                            .map(|i| {
                                let p = base_probability(i, type_count) + delta;
                                Kind::new(format!("type_{i}"), make_type_spec_complex(p))
                            })
                            .collect::<Vec<_>>()
                    },
                    |types| {
                        let eval = Evaluator::new(&types, &cache).expect("compile ok");
                        black_box(eval);
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = common::default_criterion();
    targets = evaluator_compile_benches, evaluator_run_benches
}
criterion_main!(benches);
