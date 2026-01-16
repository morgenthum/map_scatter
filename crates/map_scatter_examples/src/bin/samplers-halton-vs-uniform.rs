use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    init_tracing();
    // Common domain and base runner config
    let domain_extent = Vec2::new(100.0, 100.0);
    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let textures = TextureRegistry::new();
    let cache = FieldProgramCache::new();

    // Reproducible RNG
    let mut rng = StdRng::seed_from_u64(42);

    // Create a simple kind (always eligible, probability 1)
    let kind = make_always_placeable_kind("dots");

    // Compare Halton vs. Uniform for the same number of samples
    let count = 1000usize;

    // Run Halton
    let plan_halton = Plan::new().with_layer(Layer::new(
        "halton_layer",
        vec![kind.clone()],
        Box::new(HaltonSampling::with_rotation(count, true).with_start_index(1)),
    ));
    let mut runner_halton = ScatterRunner::try_new(config.clone(), &textures, &cache)?;
    let result_halton = runner_halton.run(&plan_halton, &mut rng);
    let _ = render_simple(
        &result_halton,
        domain_extent,
        [26, 28, 35],
        "samplers-halton-vs-uniform-halton.png",
        [240, 235, 200],
    );

    // Run Uniform (i.i.d.)
    let plan_uniform = Plan::new().with_layer(Layer::new(
        "uniform_layer",
        vec![kind],
        Box::new(UniformRandomSampling::new(count)),
    ));
    let mut runner_uniform = ScatterRunner::try_new(config.clone(), &textures, &cache)?;
    let result_uniform = runner_uniform.run(&plan_uniform, &mut rng);
    let _ = render_simple(
        &result_uniform,
        domain_extent,
        [30, 26, 26],
        "samplers-halton-vs-uniform-uniform.png",
        [200, 235, 240],
    );

    Ok(())
}

fn make_always_placeable_kind(name: &str) -> Kind {
    let mut spec = FieldGraphSpec::default();
    spec.add_with_semantics("gate", NodeSpec::constant(1.0), FieldSemantics::Gate);
    spec.add_with_semantics(
        "probability",
        NodeSpec::constant(1.0),
        FieldSemantics::Probability,
    );
    Kind::new(name, spec)
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
    rc.set_kind_style("dots", KindStyle::Circle { color, radius: 2 });

    render_run_result_to_png(result, &rc, out_path)?;
    Ok(())
}
