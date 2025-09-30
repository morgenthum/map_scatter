use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    init_tracing();
    // Domain and output image
    let domain_extent = Vec2::new(100.0, 100.0);
    let image_size = (1000, 1000);

    // Exclusion zone texture:
    // Returns 1.0 inside a disk (excluded area), 0.0 outside (allowed area).
    let exclusion = ExclusionDiskTexture { radius: 20.0 };

    let mut textures = TextureRegistry::new();
    textures.register("exclusion_disk", exclusion);

    // Field graph:
    // - inside_raw = texture("exclusion_disk") in {0,1}
    // - outside = invert(inside_raw) -> 1 outside disk (allowed), 0 inside disk (excluded)
    // - probability = clamp(outside, 0, 1)
    //
    // This yields zero probability inside the exclusion disk and full probability elsewhere.
    let dots = make_dots_kind();

    // Use Poisson-disk sampling to generate a pleasant distribution outside the exclusion zone.
    let plan = Plan::new().with_layer(Layer::new(
        "exclusion_gate",
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

    // Render result
    let mut rc = RenderConfig::new(image_size, domain_extent).with_background([235, 235, 240]);
    rc.set_kind_style(
        "dots",
        KindStyle::Circle {
            color: [30, 30, 30],
            radius: 3,
        },
    );

    let out = "fields-gate-exclusion-zone.png";
    render_run_result_to_png(&result, &rc, out)?;

    Ok(())
}

fn make_dots_kind() -> Kind {
    let mut spec = FieldGraphSpec::default();
    // Field graph:
    // - inside_raw = texture("exclusion_disk") in {0,1}
    // - outside = invert(inside_raw) -> 1 outside disk (allowed), 0 inside disk (excluded)
    // - probability = clamp(outside, 0, 1)
    spec.add(
        "inside_raw",
        NodeSpec::texture("exclusion_disk", TextureChannel::R),
    );
    spec.add("outside", NodeSpec::invert("inside_raw".into()));
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("outside".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );
    // Also mark the "outside" field as a gate to document intent.
    spec.set_semantics("outside", FieldSemantics::Gate);
    Kind::new("dots", spec)
}

// A simple procedural texture that returns 1.0 inside a disk of given radius (exclusion zone),
// and 0.0 outside. The disk is centered at the world origin, which is the center of the domain.
struct ExclusionDiskTexture {
    radius: f32,
}

impl Texture for ExclusionDiskTexture {
    fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
        let r = p.length();
        if r <= self.radius {
            1.0
        } else {
            0.0
        }
    }
}
