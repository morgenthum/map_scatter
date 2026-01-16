use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// A simple procedural texture that returns 1.0 inside a disk (gate open),
/// and 0.0 outside (gate closed). This is used to gate placements.
struct DiskMaskTexture {
    center: Vec2,
    radius: f32,
}

impl Texture for DiskMaskTexture {
    fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
        let d2 = (p - self.center).length_squared();
        let r2 = self.radius * self.radius;
        if d2 <= r2 {
            1.0
        } else {
            0.0
        }
    }
}

fn main() -> anyhow::Result<()> {
    init_tracing();
    // Domain and disk parameters
    let domain_extent = Vec2::new(100.0, 100.0);
    let disk_center = Vec2::new(0.0, 0.0);
    let disk_radius = 30.0;

    // Register the disk texture
    let mut textures = TextureRegistry::new();
    textures.register(
        "disk_mask",
        DiskMaskTexture {
            center: disk_center,
            radius: disk_radius,
        },
    );

    // Build kind via helper
    let kind = inside_disk_kind();

    // Minimal plan: one layer, Poisson-disk sampler
    let plan = Plan::new().with_layer(Layer::new(
        "inside_disk",
        vec![kind],
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

    Ok(())
}

fn inside_disk_kind() -> Kind {
    // gate = texture("disk_mask"), probability = 1.0 inside the disk
    let mut spec = FieldGraphSpec::default();
    spec.add("gate", NodeSpec::texture("disk_mask", TextureChannel::R));
    spec.set_semantics("gate", FieldSemantics::Gate);
    spec.add_with_semantics(
        "probability",
        NodeSpec::constant(1.0),
        FieldSemantics::Probability,
    );
    Kind::new("dots", spec)
}

fn render(result: &RunResult, domain_extent: Vec2) -> anyhow::Result<()> {
    let image_size = (1000, 1000);
    let background = [240, 240, 244];

    let mut rc = RenderConfig::new(image_size, domain_extent).with_background(background);
    rc.set_kind_style(
        "dots",
        KindStyle::Circle {
            color: [30, 144, 255],
            radius: 3,
        },
    );

    let out = "textures-mask-disk-basic.png";
    render_run_result_to_png(result, &rc, out)?;
    Ok(())
}
