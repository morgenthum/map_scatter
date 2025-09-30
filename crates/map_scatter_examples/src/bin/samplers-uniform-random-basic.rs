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

    // Kind constructed via helper for consistency
    let dots = make_always_placeable_kind("dots");

    // Use uniform random sampling as a fast i.i.d. baseline.
    // Increase/decrease count to adjust density.
    let count = 1000usize;
    let plan = Plan::new().with_layer(Layer::new(
        "uniform_random_layer",
        vec![dots],
        Box::new(UniformRandomSampling::new(count)),
    ));

    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let mut cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(2025);

    // No external textures required for this example
    let textures = TextureRegistry::new();

    // Run the scatter
    let mut runner = ScatterRunner::try_new(config, &textures, &mut cache)?;
    let result = runner.run(&plan, &mut rng);

    // Render to PNG
    let mut rc = RenderConfig::new(image_size, domain_extent).with_background([26, 26, 26]);
    rc.set_kind_style(
        "dots",
        KindStyle::Circle {
            color: [235, 235, 235],
            radius: 2,
        },
    );

    let out = "samplers-uniform-random-basic.png";
    render_run_result_to_png(&result, &rc, out)?;

    Ok(())
}

fn make_always_placeable_kind(name: &str) -> Kind {
    let mut spec = FieldGraphSpec::default();
    spec.add_with_semantics(
        "probability",
        NodeSpec::constant(1.0),
        FieldSemantics::Probability,
    );
    Kind::new(name, spec)
}
