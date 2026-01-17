use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{
    init_tracing, render_run_result_to_png, KindStyle, PngTextures, RenderConfig,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    init_tracing();
    let domain_extent = Vec2::new(100.0, 100.0);

    let mut png_textures = PngTextures::new();
    let texture_path = format!(
        "{}/assets/fields-distance-field-edge-attraction/map_scatter.png",
        env!("CARGO_MANIFEST_DIR")
    );
    png_textures.load_png(
        "texture",
        &texture_path,
        Vec2::new(-domain_extent.x * 0.5, -domain_extent.y * 0.5),
        Vec2::new(domain_extent.x, domain_extent.y),
    )?;

    let mut texture_registry = TextureRegistry::new();
    png_textures.register_all_into(&mut texture_registry);

    let d_max = 20.0; // Max distance to normalize (distance/d_max, clamped to 1)
    let min_offset = 0.0; // Minimal distance from white shapes where spawning begins
    let offset_smooth = 1.0; // Smooth ramp length after min_offset

    let dots = Kind::new("dots", dots_spec(d_max, min_offset, offset_smooth));

    let plan = Plan::new().with_layer(Layer::new(
        "dots",
        vec![dots],
        Box::new(JitterGridSampling::new(0.0, 0.5)),
    ));

    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(4);

    let cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(42);

    let mut runner = ScatterRunner::try_new(config, &texture_registry, &cache)?;
    let result = runner.run(&plan, &mut rng);

    render(&result)?;
    Ok(())
}

fn dots_spec(d_max: f32, min_offset: f32, offset_smooth: f32) -> FieldGraphSpec {
    let mut spec = FieldGraphSpec::default();

    spec.add("mask_raw", NodeSpec::texture("texture", TextureChannel::R));
    spec.add("mask_inv", NodeSpec::invert("mask_raw".into()));

    spec.add(
        "dist_norm",
        NodeSpec::edt_normalize("mask_inv".into(), 0.5, d_max),
    );

    spec.add_with_semantics(
        "outside",
        NodeSpec::invert("mask_raw".into()),
        FieldSemantics::Gate,
    );

    spec.add("near", NodeSpec::invert("dist_norm".into()));
    spec.add("near_sharp", NodeSpec::pow("near".into(), 4.0));

    let e0 = (min_offset / d_max).clamp(0.0, 1.0);
    let e1 = ((min_offset + offset_smooth) / d_max).clamp(0.0, 1.0);
    spec.add("gate_min", NodeSpec::smoothstep("dist_norm".into(), e0, e1));

    spec.add(
        "score",
        NodeSpec::mul(vec!["near_sharp".into(), "gate_min".into()]),
    );

    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    spec
}

fn render(result: &RunResult) -> anyhow::Result<()> {
    let image_size: (u32, u32) = (1000, 1000);
    let domain_extent = Vec2::new(100.0, 100.0);
    let background = [255, 255, 255];

    let mut cfg = RenderConfig::new(image_size, domain_extent).with_background(background);
    cfg.set_kind_style(
        "dots",
        KindStyle::Circle {
            color: [0, 0, 0],
            radius: 1,
        },
    );

    render_run_result_to_png(result, &cfg, "fields-distance-field-edge-attraction.png")?;
    Ok(())
}
