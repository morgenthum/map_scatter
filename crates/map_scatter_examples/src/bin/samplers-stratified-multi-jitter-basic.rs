use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    init_tracing();
    // Domain and image size
    let domain_extent = Vec2::new(100.0, 100.0);
    let image_size = (1000, 1000);

    // Minimal kind: always eligible and probability 1
    let kind = make_always_placeable_kind("dots");

    // Use stratified multi-jittered sampling for low-variance coverage
    let count = 1000usize;
    let plan = Plan::new().with_layer(Layer::new(
        "stratified_multi_jitter",
        vec![kind],
        Box::new(StratifiedMultiJitterSampling::with_rotation(count, true)),
    ));

    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(4242);

    // No external textures required for this example
    let textures = TextureRegistry::new();

    // Run the scatter
    let mut runner = ScatterRunner::try_new(config, &textures, &cache)?;
    let result = runner.run(&plan, &mut rng);

    // Render to PNG
    let mut rc = RenderConfig::new(image_size, domain_extent).with_background([26, 26, 26]);
    rc.set_kind_style(
        "dots",
        KindStyle::Circle {
            color: [238, 238, 238],
            radius: 2,
        },
    );

    let out = "samplers-stratified-multi-jitter-basic.png";
    render_run_result_to_png(&result, &rc, out)?;

    Ok(())
}

fn make_always_placeable_kind(name: &str) -> Kind {
    let mut spec = FieldGraphSpec::default();
    // Always allowed gate:
    spec.add_with_semantics("gate", NodeSpec::constant(1.0), FieldSemantics::Gate);
    // Always placeable with probability 1:
    spec.add_with_semantics(
        "probability",
        NodeSpec::constant(1.0),
        FieldSemantics::Probability,
    );
    Kind::new(name, spec)
}
