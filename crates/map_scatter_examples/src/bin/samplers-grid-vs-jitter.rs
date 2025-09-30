use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    init_tracing();
    // Domain and base config used for both runs
    let domain_extent = Vec2::new(100.0, 100.0);
    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let textures = TextureRegistry::new();
    let mut cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(42);

    // Compare two variants:
    // - Pure grid (jitter = 0.0)
    // - Fully jittered grid (jitter = 1.0)
    let cell_size = 5.0;

    // Run pure grid
    let plan_grid = build_plan(0.0, cell_size);
    let mut runner_grid = ScatterRunner::try_new(config.clone(), &textures, &mut cache)?;
    let result_grid = runner_grid.run(&plan_grid, &mut rng);
    render_simple(
        &result_grid,
        domain_extent,
        [235, 245, 255],
        "samplers-grid-vs-jitter-grid.png",
        [40, 120, 240],
    )?;

    // Run jittered grid
    let plan_jitter = build_plan(1.0, cell_size);
    let mut runner_jitter = ScatterRunner::try_new(config.clone(), &textures, &mut cache)?;
    let result_jitter = runner_jitter.run(&plan_jitter, &mut rng);
    render_simple(
        &result_jitter,
        domain_extent,
        [255, 246, 235],
        "samplers-grid-vs-jitter-jittered.png",
        [240, 140, 40],
    )?;

    Ok(())
}

fn build_plan(jitter: f32, cell_size: f32) -> Plan {
    let mut spec = FieldGraphSpec::default();
    spec.add_with_semantics(
        "probability",
        NodeSpec::constant(1.0),
        FieldSemantics::Probability,
    );

    let kind = Kind::new("dots", spec);

    Plan::new().with_layer(Layer::new(
        "grid_vs_jitter",
        vec![kind],
        Box::new(JitterGridSampling::new(jitter, cell_size)),
    ))
}

fn render_simple(
    result: &RunResult,
    domain_extent: Vec2,
    background: [u8; 3],
    out_path: &str,
    color: [u8; 3],
) -> anyhow::Result<()> {
    let image_size = (1000, 1000);

    let mut rc = RenderConfig::new(image_size, domain_extent).with_background(background);
    rc.set_kind_style("dots", KindStyle::Circle { color, radius: 3 });

    render_run_result_to_png(result, &rc, out_path)?;
    Ok(())
}
