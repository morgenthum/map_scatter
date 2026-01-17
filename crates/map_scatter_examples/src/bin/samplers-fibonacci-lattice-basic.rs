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

    // Use Fibonacci lattice sampling for very uniform point coverage.
    // Enable Cranley–Patterson rotation to decorrelate sequences across runs/chunks.
    let count = 80usize;
    let plan = Plan::new().with_layer(Layer::new(
        "fibonacci_lattice_layer",
        vec![dots],
        Box::new(FibonacciLatticeSampling::with_rotation(count, true)),
    ));

    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(12345);

    // No external textures required for this example
    let textures = TextureRegistry::new();

    // Run the scatter
    let mut runner = ScatterRunner::try_new(config, &textures, &cache)?;
    let result = runner.run(&plan, &mut rng);

    // Render to PNG
    let mut rc = RenderConfig::new(image_size, domain_extent).with_background([28, 28, 28]);
    rc.set_kind_style(
        "dots",
        KindStyle::Circle {
            color: [240, 240, 240],
            radius: 2,
        },
    );

    let out = "samplers-fibonacci-lattice-basic.png";
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
