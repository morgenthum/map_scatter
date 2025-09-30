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

    // Compare two structured samplers:
    // - Hexagonal lattice with jitter
    // - Square grid with jitter
    //
    // Both use the same jitter and a similar cell size for a fair visual comparison.
    let jitter = 0.75;
    let hex_cell = 5.0;
    // Optional: choose square cell size so that point density is roughly comparable to hex.
    // For a hex lattice with spacing 'dx', density ≈ 2/(sqrt(3)*dx^2) ≈ 1.1547/dx^2.
    // For a square grid with spacing 's', density = 1/s^2.
    // Matching densities => s ≈ dx / sqrt(1.1547) ≈ 0.9306·dx.
    let square_cell = hex_cell / 1.154_700_5_f32.sqrt();

    // Create the always-placeable kind (probability = 1)
    let kind = make_always_placeable_kind("dots");

    // Run hex jitter
    let plan_hex = Plan::new().with_layer(Layer::new(
        "hex_jitter",
        vec![kind.clone()],
        Box::new(HexJitterGridSampling::new(jitter, hex_cell)),
    ));
    let mut runner_hex = ScatterRunner::try_new(config.clone(), &textures, &mut cache)?;
    let result_hex = runner_hex.run(&plan_hex, &mut rng);
    render_simple(
        &result_hex,
        domain_extent,
        [26, 28, 35],
        "samplers-hex-vs-square-grid-hex.png",
        [240, 235, 200],
    )?;

    // Run square jitter
    let plan_square = Plan::new().with_layer(Layer::new(
        "square_jitter",
        vec![kind],
        Box::new(JitterGridSampling::new(jitter, square_cell)),
    ));
    let mut runner_square = ScatterRunner::try_new(config.clone(), &textures, &mut cache)?;
    let result_square = runner_square.run(&plan_square, &mut rng);
    render_simple(
        &result_square,
        domain_extent,
        [30, 26, 26],
        "samplers-hex-vs-square-grid-square.png",
        [200, 235, 240],
    )?;

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
