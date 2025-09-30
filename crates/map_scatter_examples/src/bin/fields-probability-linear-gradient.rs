use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    let domain_extent = Vec2::new(100.0, 100.0);

    // Register a simple procedural texture that returns the normalized x-coordinate (u in [0,1])
    // across the domain. This directly acts as our probability field.
    let mut textures = TextureRegistry::new();
    textures.register("u_gradient", LinearUTexture { domain_extent });

    // Kind constructed via helper for consistency
    let dots = gradient_kind();

    // Use a simple sampler; the field should bias placements towards the right (u close to 1).
    let plan = Plan::new().with_layer(Layer::new(
        "u_linear_gradient",
        vec![dots],
        Box::new(PoissonDiskSampling::new(2.5)),
    ));

    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let mut cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(42);
    let mut runner = ScatterRunner::try_new(config, &textures, &mut cache)?;
    let result = runner.run(&plan, &mut rng);

    // Render
    let image_size = (1000, 1000);
    let mut rc = RenderConfig::new(image_size, domain_extent).with_background([235, 235, 240]);
    rc.set_kind_style(
        "dots",
        KindStyle::Circle {
            color: [40, 120, 240],
            radius: 3,
        },
    );

    let out = "fields-probability-linear-gradient.png";
    render_run_result_to_png(&result, &rc, out)?;

    Ok(())
}

fn gradient_kind() -> Kind {
    // probability = clamp(texture("u_gradient"), 0, 1)
    let mut spec = FieldGraphSpec::default();
    spec.add("u_raw", NodeSpec::texture("u_gradient", TextureChannel::R));
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("u_raw".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );
    Kind::new("dots", spec)
}

// A procedural texture that returns the normalized x-coordinate (u) within the domain.
// u = 0 at the left edge, u = 1 at the right edge.
struct LinearUTexture {
    domain_extent: Vec2,
}

impl Texture for LinearUTexture {
    fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
        // Convert world position p.x in [-extent.x/2, extent.x/2] to u in [0,1]
        ((p.x / self.domain_extent.x) + 0.5).clamp(0.0, 1.0)
    }
}
