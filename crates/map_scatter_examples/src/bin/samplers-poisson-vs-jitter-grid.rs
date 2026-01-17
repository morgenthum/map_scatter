use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{
    init_tracing, render_run_result_to_png, KindStyle, PngTextures, RenderConfig,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    init_tracing();
    let tree_kind = tree_kind();
    let grass_kind = grass_kind();

    let plan = Plan::new()
        .with_layer(Layer::new(
            "grass",
            vec![grass_kind],
            Box::new(JitterGridSampling::new(1.0, 4.0)),
        ))
        .with_layer(Layer::new(
            "trees",
            vec![tree_kind],
            Box::new(PoissonDiskSampling::new(4.0)),
        ));

    let config = RunConfig::new(Vec2::new(100.0, 100.0))
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(4);

    let cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(42);

    let mut textures = PngTextures::new();
    let tex_path = format!(
        "{}/assets/samplers-poisson-vs-jitter-grid/trees.png",
        env!("CARGO_MANIFEST_DIR")
    );

    textures.load_png(
        "trees",
        tex_path,
        Vec2::new(-config.domain_extent.x * 0.5, -config.domain_extent.y * 0.5),
        Vec2::new(config.domain_extent.x, config.domain_extent.y),
    )?;

    let mut registry = TextureRegistry::new();
    textures.register_all_into(&mut registry);

    let mut runner = ScatterRunner::try_new(config, &registry, &cache)?;
    let result = runner.run(&plan, &mut rng);

    render(&result)?;
    Ok(())
}

fn tree_kind() -> Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add_with_semantics(
        "probability",
        NodeSpec::texture("trees", TextureChannel::R),
        FieldSemantics::Probability,
    );

    Kind::new("tree", spec)
}

fn grass_kind() -> Kind {
    let mut spec = FieldGraphSpec::default();
    spec.add(
        "trees_texture",
        NodeSpec::texture("trees", TextureChannel::R),
    )
    .add_with_semantics(
        "probability",
        NodeSpec::invert("trees_texture".into()),
        FieldSemantics::Probability,
    );

    Kind::new("grass", spec)
}

fn render(result: &RunResult) -> anyhow::Result<()> {
    let image_size = (1000, 1000);
    let domain_extent = Vec2::new(100.0, 100.0);
    let background = [220, 220, 220];

    let mut config = RenderConfig::new(image_size, domain_extent).with_background(background);

    config
        .set_kind_style(
            "grass",
            KindStyle::Circle {
                color: [0, 255, 0],
                radius: 3,
            },
        )
        .set_kind_style(
            "tree",
            KindStyle::Circle {
                color: [139, 69, 19],
                radius: 7,
            },
        );

    let out = "samplers-poisson-vs-jitter-grid.png";
    render_run_result_to_png(result, &config, out)?;
    Ok(())
}
