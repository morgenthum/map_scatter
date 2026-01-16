use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() -> anyhow::Result<()> {
    init_tracing();
    let domain_extent = Vec2::new(100.0, 100.0);
    let grid = SingleChannelGrid::radial(domain_extent, 64, 64);
    let mut textures = TextureRegistry::new();
    textures.register("grid_a", grid);
    let kind = make_radial_kind();
    let plan = Plan::new().with_layer(Layer::new(
        "layer_dots",
        vec![kind],
        Box::new(PoissonDiskSampling::new(2.5)),
    ));
    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(123);
    let mut runner = ScatterRunner::try_new(config, &textures, &cache)?;
    let result = runner.run(&plan, &mut rng);
    render(&result)?;
    Ok(())
}

fn make_radial_kind() -> Kind {
    let mut spec = FieldGraphSpec::default();
    spec.add("grid_tex", NodeSpec::texture("grid_a", TextureChannel::R));
    spec.add("probability", NodeSpec::clamp("grid_tex".into(), 0.0, 1.0));
    spec.set_semantics("probability", FieldSemantics::Probability);
    Kind::new("dots", spec)
}

fn render(result: &RunResult) -> anyhow::Result<()> {
    let image_size = (1000, 1000);
    let domain_extent = Vec2::new(100.0, 100.0);
    let background = [0, 0, 0];

    let mut config = RenderConfig::new(image_size, domain_extent).with_background(background);

    config.set_kind_style(
        "dots",
        KindStyle::Circle {
            color: [255, 255, 255],
            radius: 1,
        },
    );

    let out = "grids-radial-probability.png";
    render_run_result_to_png(result, &config, out)?;
    Ok(())
}

#[derive(Clone)]
struct SingleChannelGrid {
    origin: Vec2,
    extent: Vec2,
    width: u32,
    height: u32,
    data: Vec<f32>,
}

impl SingleChannelGrid {
    fn radial(domain_extent: Vec2, width: u32, height: u32) -> Self {
        let origin = Vec2::new(-domain_extent.x * 0.5, -domain_extent.y * 0.5);
        let extent = domain_extent;

        let mut data = vec![0.0; (width as usize) * (height as usize)];

        for y in 0..height {
            for x in 0..width {
                let u = (x as f32 + 0.5) / width as f32;
                let v = (y as f32 + 0.5) / height as f32;
                let wx = origin.x + u * extent.x;
                let wy = origin.y + v * extent.y;

                let r = (wx * wx + wy * wy).sqrt();
                let r_max = 0.5 * domain_extent.length();
                let val = (1.0 - r / r_max).clamp(0.0, 1.0);

                data[(y as usize) * (width as usize) + (x as usize)] = val;
            }
        }

        Self {
            origin,
            extent,
            width,
            height,
            data,
        }
    }

    fn sample_nearest(&self, p: Vec2) -> f32 {
        if self.width == 0 || self.height == 0 {
            return 0.0;
        }
        let u = if self.extent.x != 0.0 {
            ((p.x - self.origin.x) / self.extent.x).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let v = if self.extent.y != 0.0 {
            ((p.y - self.origin.y) / self.extent.y).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let x = ((u * self.width as f32) as u32).min(self.width - 1);
        let y = ((v * self.height as f32) as u32).min(self.height - 1);
        self.data[(y as usize) * (self.width as usize) + (x as usize)]
    }
}

impl Texture for SingleChannelGrid {
    fn sample(&self, channel: TextureChannel, p: Vec2) -> f32 {
        match channel {
            TextureChannel::R => self.sample_nearest(p),
            TextureChannel::A => 1.0,
            _ => 0.0,
        }
    }
}
