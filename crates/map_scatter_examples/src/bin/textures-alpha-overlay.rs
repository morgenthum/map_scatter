use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// A base procedural texture in \[0,1\] using a smooth sinusoidal pattern.
/// Encodes variation across the domain to act as a weak probability base.
struct BasePatternTexture {
    domain_extent: Vec2,
    freq_u: f32,
    freq_v: f32,
}

impl Texture for BasePatternTexture {
    fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
        // Map world position to [0,1]^2
        let u = ((p.x / self.domain_extent.x) + 0.5).clamp(0.0, 1.0);
        let v = ((p.y / self.domain_extent.y) + 0.5).clamp(0.0, 1.0);

        // Smooth pattern in [0,1]
        (0.5 + 0.5
            * ((2.0 * std::f32::consts::PI * self.freq_u * u).sin()
                * (2.0 * std::f32::consts::PI * self.freq_v * v).cos()))
        .clamp(0.0, 1.0)
    }
}

/// A texture that returns the normalized distance from a center:
/// r_norm = clamp(length(p - center) / radius, 0, 1)
/// We'll turn this into an alpha blob in the field-graph using smoothstep and invert.
struct OverlayRadiusTexture {
    center: Vec2,
    radius: f32,
}

impl Texture for OverlayRadiusTexture {
    fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
        let d = (p - self.center).length();
        (d / self.radius).clamp(0.0, 1.0)
    }
}

fn main() -> anyhow::Result<()> {
    init_tracing();
    let domain_extent = Vec2::new(100.0, 100.0);

    // Register procedural textures
    let mut textures = TextureRegistry::new();
    textures.register(
        "base_pattern",
        BasePatternTexture {
            domain_extent,
            freq_u: 2.0,
            freq_v: 1.0,
        },
    );
    textures.register(
        "overlay_r",
        OverlayRadiusTexture {
            center: Vec2::new(10.0, -5.0),
            radius: 30.0,
        },
    );

    // Field graph constructed via helper for consistency
    let kind = make_dots_kind();

    // Plan: single layer, Poisson-disk sampler
    let plan = Plan::new().with_layer(Layer::new(
        "alpha_overlay",
        vec![kind],
        Box::new(PoissonDiskSampling::new(2.2)),
    ));

    // Runner
    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let mut cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(42);

    let mut runner = ScatterRunner::try_new(config, &textures, &mut cache)?;
    let result = runner.run(&plan, &mut rng);

    // Render
    render(&result, domain_extent)?;

    Ok(())
}

fn render(result: &RunResult, domain_extent: Vec2) -> anyhow::Result<()> {
    let image_size = (1000, 1000);
    let background = [242, 242, 246];

    let mut rc = RenderConfig::new(image_size, domain_extent).with_background(background);
    rc.set_kind_style(
        "dots",
        KindStyle::Circle {
            color: [30, 120, 220],
            radius: 3,
        },
    );

    let out = "textures-alpha-overlay.png";
    render_run_result_to_png(result, &rc, out)?;
    Ok(())
}

fn make_dots_kind() -> Kind {
    // Field graph:
    // - base_prob = clamp(texture(base_pattern), 0, 1)
    // - r_norm = texture(overlay_r) in [0,1], 0 at center, 1 at/beyond radius
    // - overlay_alpha_raw = smoothstep(r_norm, e0=0.25, e1=0.45)  -> 0 near center, 1 outside
    // - overlay_alpha = invert(overlay_alpha_raw)                  -> 1 near center, 0 outside
    // - overlay_scaled = scale(overlay_alpha, 0.8)                 -> alpha-strength
    // - combined = max(base_prob, overlay_scaled)                  -> overlay "adds" emphasis
    // - probability = min(combined, cap=0.90)                      -> demonstrate min-cap
    let mut spec = FieldGraphSpec::default();

    // Base
    spec.add(
        "base_raw",
        NodeSpec::texture("base_pattern", TextureChannel::R),
    );
    spec.add("base_prob", NodeSpec::clamp("base_raw".into(), 0.0, 1.0));

    // Overlay alpha construction
    spec.add("r_norm", NodeSpec::texture("overlay_r", TextureChannel::R));
    spec.add(
        "overlay_alpha_raw",
        NodeSpec::smoothstep("r_norm".into(), 0.25, 0.45),
    );
    spec.add(
        "overlay_alpha",
        NodeSpec::invert("overlay_alpha_raw".into()),
    );
    spec.add(
        "overlay_scaled",
        NodeSpec::scale("overlay_alpha".into(), 0.8),
    );

    // Combine using max, then cap with min to showcase both ops
    spec.add(
        "combined",
        NodeSpec::max(vec!["base_prob".into(), "overlay_scaled".into()]),
    );
    spec.add("cap", NodeSpec::constant(0.90));
    spec.add(
        "capped",
        NodeSpec::min(vec!["combined".into(), "cap".into()]),
    );

    // Final probability
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("capped".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    Kind::new("dots", spec)
}
