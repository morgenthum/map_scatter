use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// A simple procedural texture that encodes different spatial patterns in different channels:
/// - R channel: left-to-right gradient (u in \[0,1\]).
/// - G channel: bottom-to-top gradient (v in \[0,1\]).
/// - B/A: unused in this example (return 0.0).
struct SplitChannelsTexture {
    domain_extent: Vec2,
}

impl Texture for SplitChannelsTexture {
    fn sample(&self, channel: TextureChannel, p: Vec2) -> f32 {
        let u = ((p.x / self.domain_extent.x) + 0.5).clamp(0.0, 1.0);
        let v = ((p.y / self.domain_extent.y) + 0.5).clamp(0.0, 1.0);
        match channel {
            TextureChannel::R => u, // Bias to the right
            TextureChannel::G => v, // Bias to the top
            TextureChannel::B | TextureChannel::A => 0.0,
        }
    }
}

fn main() -> anyhow::Result<()> {
    init_tracing();
    let domain_extent = Vec2::new(100.0, 100.0);

    // Register one texture that provides multiple channels with different patterns.
    let mut textures = TextureRegistry::new();
    textures.register("split", SplitChannelsTexture { domain_extent });

    // Two kinds that each consume a different channel of the same texture.
    let red = red_kind();
    let green = green_kind();

    // One layer with both kinds using a Poisson-disk sampler.
    let plan = Plan::new().with_layer(Layer::new(
        "channel_split_layer",
        vec![red, green],
        Box::new(PoissonDiskSampling::new(2.5)),
    ));

    // Runner config
    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(42);

    // Run
    let mut runner = ScatterRunner::try_new(config, &textures, &cache)?;
    let result = runner.run(&plan, &mut rng);

    // Render to PNG
    render(&result, domain_extent)?;
    // Removed println; centralized info logs are emitted by render_run_result_to_png
    Ok(())
}

fn red_kind() -> Kind {
    // probability = clamp(texture(split.R), 0, 1)
    let mut spec = FieldGraphSpec::default();
    spec.add("r_raw", NodeSpec::texture("split", TextureChannel::R));
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("r_raw".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );
    Kind::new("red", spec)
}

fn green_kind() -> Kind {
    // probability = clamp(texture(split.G), 0, 1)
    let mut spec = FieldGraphSpec::default();
    spec.add("g_raw", NodeSpec::texture("split", TextureChannel::G));
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("g_raw".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );
    Kind::new("green", spec)
}

fn render(result: &RunResult, domain_extent: Vec2) -> anyhow::Result<()> {
    let image_size = (1000, 1000);
    let background = [240, 240, 244];

    let mut rc = RenderConfig::new(image_size, domain_extent).with_background(background);
    rc.set_kind_style(
        "red",
        KindStyle::Circle {
            color: [220, 30, 30],
            radius: 3,
        },
    );
    rc.set_kind_style(
        "green",
        KindStyle::Circle {
            color: [20, 180, 60],
            radius: 3,
        },
    );

    let out = "textures-channel-split.png";
    render_run_result_to_png(result, &rc, out)?;
    Ok(())
}
