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

    // Cluster process parameters
    // With density-based parents: expected parents = density * area
    // Total expected points ≈ parents * mean_children
    let parent_density = 0.02_f32; // parents per unit area
    let mean_children = 6.0_f32;

    // Thomas (Gaussian kernel) parameters
    let sigma = 2.0_f32;

    // Neyman–Scott (uniform disk kernel) parameters
    let radius = 2.0_f32;

    // Run Thomas process
    let plan_thomas = Plan::new().with_layer(Layer::new(
        "clustered_thomas",
        vec![kind.clone()],
        Box::new(ClusteredSampling::thomas_with_density(
            parent_density,
            mean_children,
            sigma,
        )),
    ));
    let mut runner_thomas = ScatterRunner::try_new(config.clone(), &textures, &cache)?;
    let result_thomas = runner_thomas.run(&plan_thomas, &mut rng);
    render_simple(
        &result_thomas,
        domain_extent,
        [26, 28, 35],
        "samplers-clustered-thomas.png",
        [240, 235, 200],
    )?;

    // Run Neyman–Scott process
    let plan_ns = Plan::new().with_layer(Layer::new(
        "clustered_neyman_scott",
        vec![kind],
        Box::new(ClusteredSampling::neyman_scott_with_density(
            parent_density,
            mean_children,
            radius,
        )),
    ));
    let mut runner_ns = ScatterRunner::try_new(config.clone(), &textures, &cache)?;
    let result_ns = runner_ns.run(&plan_ns, &mut rng);
    render_simple(
        &result_ns,
        domain_extent,
        [30, 26, 26],
        "samplers-clustered-neyman-scott.png",
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
