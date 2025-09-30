use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{
    init_tracing, render_run_result_to_png, KindStyle, PngTextures, RenderConfig,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    init_tracing();
    let red_kind = red_kind();
    let green_kind = green_kind();

    let plan = Plan::new()
        .with_layer(Layer::new(
            "grass",
            vec![green_kind],
            Box::new(PoissonDiskSampling::new(1.0)),
        ))
        .with_layer(Layer::new(
            "trees",
            vec![red_kind],
            Box::new(PoissonDiskSampling::new(1.0)),
        ));

    let config = RunConfig::new(Vec2::new(100.0, 100.0))
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(4);

    let mut cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(42);

    let mut textures = PngTextures::new();
    let no_space_path = format!(
        "{}/assets/textures-splatmap-masked-scatter/no-space.png",
        env!("CARGO_MANIFEST_DIR")
    );
    let splat_path = format!(
        "{}/assets/textures-splatmap-masked-scatter/splat.png",
        env!("CARGO_MANIFEST_DIR")
    );

    textures.load_png(
        "no_space",
        no_space_path,
        Vec2::new(-config.domain_extent.x * 0.5, -config.domain_extent.y * 0.5),
        Vec2::new(config.domain_extent.x, config.domain_extent.y),
    )?;

    textures.load_png(
        "splat",
        splat_path,
        Vec2::new(-config.domain_extent.x * 0.5, -config.domain_extent.y * 0.5),
        Vec2::new(config.domain_extent.x, config.domain_extent.y),
    )?;

    let mut registry = TextureRegistry::new();
    textures.register_all_into(&mut registry);

    let mut runner = ScatterRunner::try_new(config, &registry, &mut cache)?;
    let result = runner.run(&plan, &mut rng);

    render(&result)?;
    Ok(())
}

fn red_kind() -> Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add(
        "no_space_raw",
        NodeSpec::texture("no_space", TextureChannel::R),
    )
    .add("no_space_inverted", NodeSpec::invert("no_space_raw".into()))
    .add("splat_raw", NodeSpec::texture("splat", TextureChannel::R))
    .add_with_semantics(
        "probability",
        NodeSpec::mul(vec!["no_space_inverted".into(), "splat_raw".into()]),
        FieldSemantics::Probability,
    );

    Kind::new("red", spec)
}

fn green_kind() -> Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add(
        "no_space_raw",
        NodeSpec::texture("no_space", TextureChannel::R),
    )
    .add("no_space_inverted", NodeSpec::invert("no_space_raw".into()))
    .add("splat_raw", NodeSpec::texture("splat", TextureChannel::G))
    .add_with_semantics(
        "probability",
        NodeSpec::mul(vec!["no_space_inverted".into(), "splat_raw".into()]),
        FieldSemantics::Probability,
    );

    Kind::new("green", spec)
}

fn render(result: &RunResult) -> anyhow::Result<()> {
    let image_size = (1000, 1000);
    let domain_extent = Vec2::new(100.0, 100.0);
    let background = [220, 220, 220];

    let mut config = RenderConfig::new(image_size, domain_extent).with_background(background);

    config
        .set_kind_style(
            "red",
            KindStyle::Circle {
                color: [255, 0, 0],
                radius: 2,
            },
        )
        .set_kind_style(
            "green",
            KindStyle::Circle {
                color: [0, 255, 19],
                radius: 2,
            },
        );

    let out_path = "textures-splatmap-masked-scatter.png";
    render_run_result_to_png(result, &config, out_path)?;
    Ok(())
}
