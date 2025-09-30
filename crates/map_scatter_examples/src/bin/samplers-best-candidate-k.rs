use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    // Common domain and base runner config
    let domain_extent = Vec2::new(100.0, 100.0);
    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let textures = TextureRegistry::new();
    let mut cache = FieldProgramCache::new();

    // Reproducible RNG
    let mut rng = StdRng::seed_from_u64(42);

    // Create a simple kind (always eligible, probability 1)
    let kind = make_always_placeable_kind("dots");

    // Compare BestCandidate with two different k values (trade-off: quality vs. speed)
    let count = 1000usize;
    let k_low = 8usize;
    let k_high = 32usize;

    // Run BestCandidate with k_low
    let plan_low = Plan::new().with_layer(Layer::new(
        "best_candidate_k_low",
        vec![kind.clone()],
        Box::new(BestCandidateSampling::new(count, k_low)),
    ));
    let mut runner_low = ScatterRunner::try_new(config.clone(), &textures, &mut cache)?;
    let result_low = runner_low.run(&plan_low, &mut rng);
    render_simple(
        &result_low,
        domain_extent,
        [26, 28, 35],
        "samplers-best-candidate-k-low.png",
        [240, 235, 200],
    )?;

    // Run BestCandidate with k_high
    let plan_high = Plan::new().with_layer(Layer::new(
        "best_candidate_k_high",
        vec![kind],
        Box::new(BestCandidateSampling::new(count, k_high)),
    ));
    let mut runner_high = ScatterRunner::try_new(config.clone(), &textures, &mut cache)?;
    let result_high = runner_high.run(&plan_high, &mut rng);
    render_simple(
        &result_high,
        domain_extent,
        [30, 26, 26],
        "samplers-best-candidate-k-high.png",
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
