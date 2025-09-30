use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Coarse single-channel grid data (values in \[0,1\]) with helpers for sampling.
#[derive(Clone)]
struct GridData {
    origin: Vec2,
    extent: Vec2,
    width: u32,
    height: u32,
    data: Vec<f32>,
}

impl GridData {
    /// Build a coarse grid that mixes a tilted gradient with some low-frequency waves.
    /// This makes nearest sampling appear blocky, while bilinear looks smooth.
    fn coarse_gradient_with_waves(domain_extent: Vec2, width: u32, height: u32) -> Self {
        let origin = Vec2::new(-domain_extent.x * 0.5, -domain_extent.y * 0.5);
        let extent = domain_extent;
        let mut data = vec![0.0; (width as usize) * (height as usize)];

        for y in 0..height {
            for x in 0..width {
                // Texel center UVs
                let u = (x as f32 + 0.5) / (width as f32);
                let v = (y as f32 + 0.5) / (height as f32);

                // Tilted gradient + low-frequency waves
                let base = (0.75 * u + 0.25 * v).clamp(0.0, 1.0);
                let waves = 0.25
                    * ((2.0 * std::f32::consts::PI * 3.0 * u).sin()
                        * (2.0 * std::f32::consts::PI * 2.0 * v).cos());
                let val = (base + waves).clamp(0.0, 1.0);

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

    #[inline]
    fn sample_nearest(&self, p: Vec2) -> f32 {
        if self.width == 0 || self.height == 0 {
            return 0.0;
        }

        // Map world pos to [0,1]^2 in the grid
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

        // Nearest texel by partitioning into width/height bins
        let xi = ((u * self.width as f32) as u32).min(self.width - 1);
        let yi = ((v * self.height as f32) as u32).min(self.height - 1);
        self.data[(yi as usize) * (self.width as usize) + (xi as usize)]
    }

    #[inline]
    fn sample_bilinear(&self, p: Vec2) -> f32 {
        if self.width == 0 || self.height == 0 {
            return 0.0;
        }

        // Map world pos to continuous texel coordinates
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

        // Continuous coordinates over texel indices [0..width-1], [0..height-1]
        let x = u * (self.width.saturating_sub(1) as f32);
        let y = v * (self.height.saturating_sub(1) as f32);

        let x0 = x.floor().clamp(0.0, (self.width - 1) as f32) as u32;
        let y0 = y.floor().clamp(0.0, (self.height - 1) as f32) as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let fx = (x - x0 as f32).clamp(0.0, 1.0);
        let fy = (y - y0 as f32).clamp(0.0, 1.0);

        let idx =
            |ix: u32, iy: u32| -> usize { (iy as usize) * (self.width as usize) + (ix as usize) };
        let v00 = self.data[idx(x0, y0)];
        let v10 = self.data[idx(x1, y0)];
        let v01 = self.data[idx(x0, y1)];
        let v11 = self.data[idx(x1, y1)];

        let vx0 = v00 * (1.0 - fx) + v10 * fx;
        let vx1 = v01 * (1.0 - fx) + v11 * fx;
        vx0 * (1.0 - fy) + vx1 * fy
    }
}

/// A Texture wrapper that samples the grid using nearest neighbor.
#[derive(Clone)]
struct NearestGridTexture {
    grid: GridData,
}

impl Texture for NearestGridTexture {
    fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
        self.grid.sample_nearest(p)
    }
}

/// A Texture wrapper that samples the grid using bilinear interpolation.
#[derive(Clone)]
struct BilinearGridTexture {
    grid: GridData,
}

impl Texture for BilinearGridTexture {
    fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
        self.grid.sample_bilinear(p)
    }
}

fn make_nearest_kind() -> Kind {
    let mut spec = FieldGraphSpec::default();
    spec.add("g", NodeSpec::texture("grid_nearest", TextureChannel::R));
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("g".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );
    Kind::new("nearest", spec)
}

fn make_bilinear_kind() -> Kind {
    let mut spec = FieldGraphSpec::default();
    spec.add("g", NodeSpec::texture("grid_bilinear", TextureChannel::R));
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("g".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );
    Kind::new("bilinear", spec)
}

fn main() -> anyhow::Result<()> {
    init_tracing();
    // Domain and a coarse grid to highlight sampling differences
    let domain_extent = Vec2::new(100.0, 100.0);
    let grid = GridData::coarse_gradient_with_waves(domain_extent, 32, 32);

    // Register the same grid twice with different sampling strategies
    let mut textures = TextureRegistry::new();
    textures.register("grid_nearest", NearestGridTexture { grid: grid.clone() });
    textures.register("grid_bilinear", BilinearGridTexture { grid });

    // Build two simple specs via helpers:
    let nearest_kind = make_nearest_kind();
    let bilinear_kind = make_bilinear_kind();

    // Shared runner config
    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let mut cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(42);

    // Run nearest
    let nearest_plan = Plan::new().with_layer(Layer::new(
        "nearest_layer",
        vec![nearest_kind],
        Box::new(PoissonDiskSampling::new(2.4)),
    ));
    let mut runner = ScatterRunner::try_new(config, &textures, &mut cache)?;
    let nearest_result = runner.run(&nearest_plan, &mut rng);
    render_nearest(&nearest_result, domain_extent)?;

    // Run bilinear
    let bilinear_plan = Plan::new().with_layer(Layer::new(
        "bilinear_layer",
        vec![bilinear_kind],
        Box::new(PoissonDiskSampling::new(2.4)),
    ));
    // Reuse runner (same config/cache/registry), just run with the new plan
    let bilinear_result = runner.run(&bilinear_plan, &mut rng);
    render_bilinear(&bilinear_result, domain_extent)?;
    Ok(())
}

fn render_nearest(result: &RunResult, domain_extent: Vec2) -> anyhow::Result<()> {
    let image_size = (1000, 1000);
    let mut rc = RenderConfig::new(image_size, domain_extent).with_background([236, 238, 242]);
    rc.set_kind_style(
        "nearest",
        KindStyle::Circle {
            color: [40, 120, 240],
            radius: 3,
        },
    );
    render_run_result_to_png(result, &rc, "grids-bilinear-vs-nearest-nearest.png")?;
    Ok(())
}

fn render_bilinear(result: &RunResult, domain_extent: Vec2) -> anyhow::Result<()> {
    let image_size = (1000, 1000);
    let mut rc = RenderConfig::new(image_size, domain_extent).with_background([236, 238, 242]);
    rc.set_kind_style(
        "bilinear",
        KindStyle::Circle {
            color: [40, 180, 100],
            radius: 3,
        },
    );
    render_run_result_to_png(result, &rc, "grids-bilinear-vs-nearest-bilinear.png")?;
    Ok(())
}
