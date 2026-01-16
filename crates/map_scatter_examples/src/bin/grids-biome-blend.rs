use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// A simple single-channel grid that we use as a Texture source.
/// We'll generate two grids: elevation and moisture (both in \[0,1\]).
#[derive(Clone)]
struct SingleChannelGrid {
    origin: Vec2,
    extent: Vec2,
    width: u32,
    height: u32,
    data: Vec<f32>,
}

impl SingleChannelGrid {
    /// Generate a pseudo elevation map:
    /// - A radial slope (higher towards the center)
    /// - Low-frequency sinusoidal variation
    ///   final = clamp(0.6 * radial + 0.4 * sinus, 0, 1)
    fn elevation(domain_extent: Vec2, width: u32, height: u32) -> Self {
        let origin = Vec2::new(-domain_extent.x * 0.5, -domain_extent.y * 0.5);
        let extent = domain_extent;

        let mut data = vec![0.0; (width as usize) * (height as usize)];
        let r_max = 0.5 * domain_extent.length();

        for y in 0..height {
            for x in 0..width {
                let u = (x as f32 + 0.5) / width as f32;
                let v = (y as f32 + 0.5) / height as f32;
                let wx = origin.x + u * extent.x;
                let wy = origin.y + v * extent.y;

                let r = (wx * wx + wy * wy).sqrt();
                let radial = (1.0 - r / r_max).clamp(0.0, 1.0);

                let sinus = 0.5
                    + 0.5
                        * ((2.0 * std::f32::consts::PI * 1.4 * u).sin()
                            * (2.0 * std::f32::consts::PI * 0.9 * v).cos());

                let h = (0.6 * radial + 0.4 * sinus).clamp(0.0, 1.0);
                data[(y as usize) * (width as usize) + (x as usize)] = h;
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

    /// Generate a pseudo moisture map:
    /// - A left-to-right gradient (wetter on the left)
    /// - Low-frequency sinusoidal variation
    ///   final = clamp(0.6 * (1 - u) + 0.4 * sinus, 0, 1)
    fn moisture(domain_extent: Vec2, width: u32, height: u32) -> Self {
        let origin = Vec2::new(-domain_extent.x * 0.5, -domain_extent.y * 0.5);
        let extent = domain_extent;

        let mut data = vec![0.0; (width as usize) * (height as usize)];

        for y in 0..height {
            for x in 0..width {
                let u = (x as f32 + 0.5) / width as f32;
                let v = (y as f32 + 0.5) / height as f32;
                let _wx = origin.x + u * extent.x;
                let _wy = origin.y + v * extent.y;

                let gradient = 1.0 - u; // wetter on the left
                let sinus = 0.5
                    + 0.5
                        * ((2.0 * std::f32::consts::PI * 0.8 * u).sin()
                            * (2.0 * std::f32::consts::PI * 1.1 * v).cos());

                let m = (0.6 * gradient + 0.4 * sinus).clamp(0.0, 1.0);
                data[(y as usize) * (width as usize) + (x as usize)] = m;
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

fn main() -> anyhow::Result<()> {
    init_tracing();
    // Domain
    let domain_extent = Vec2::new(100.0, 100.0);

    // Bake grids (common gamedev workflow: heightmap + moisture map)
    let elev = SingleChannelGrid::elevation(domain_extent, 256, 256);
    let moist = SingleChannelGrid::moisture(domain_extent, 256, 256);

    // Register as textures
    let mut textures = TextureRegistry::new();
    textures.register("elevation", elev);
    textures.register("moisture", moist);

    // Biome-like kinds driven by elevation + moisture:
    // - water: low elevation, high moisture
    // - desert: mid elevation, low moisture
    // - forest: mid elevation, high moisture
    // - mountain: high elevation (less moisture bias)
    let water = kind_water();
    let desert = kind_desert();
    let forest = kind_forest();
    let mountain = kind_mountain();

    // Plan: single layer with multiple kinds
    let plan = Plan::new().with_layer(Layer::new(
        "biome_blend",
        vec![water, desert, forest, mountain],
        Box::new(PoissonDiskSampling::new(2.2)),
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

    // Render
    render(&result)?;

    Ok(())
}

// Low elevation + high moisture
fn kind_water() -> Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add(
        "elev_raw",
        NodeSpec::texture("elevation", TextureChannel::R),
    );
    spec.add(
        "moist_raw",
        NodeSpec::texture("moisture", TextureChannel::R),
    );

    // Low elevation mask ~ 1 when elev is small
    spec.add(
        "elev_low_rise",
        NodeSpec::smoothstep("elev_raw".into(), 0.15, 0.25),
    );
    spec.add("elev_low", NodeSpec::invert("elev_low_rise".into()));

    // High moisture
    spec.add(
        "moist_high",
        NodeSpec::smoothstep("moist_raw".into(), 0.50, 0.70),
    );

    spec.add_with_semantics(
        "probability",
        NodeSpec::mul(vec!["elev_low".into(), "moist_high".into()]),
        FieldSemantics::Probability,
    );

    Kind::new("water", spec)
}

// Mid elevation + low moisture
fn kind_desert() -> Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add(
        "elev_raw",
        NodeSpec::texture("elevation", TextureChannel::R),
    );
    spec.add(
        "moist_raw",
        NodeSpec::texture("moisture", TextureChannel::R),
    );

    // Elevation within mid band:
    // above water
    spec.add(
        "elev_above_water",
        NodeSpec::smoothstep("elev_raw".into(), 0.20, 0.30),
    );
    // below mountain (invert high-elev)
    spec.add(
        "elev_high_rise",
        NodeSpec::smoothstep("elev_raw".into(), 0.75, 0.90),
    );
    spec.add(
        "elev_below_mountain",
        NodeSpec::invert("elev_high_rise".into()),
    );
    spec.add(
        "elev_mid",
        NodeSpec::mul(vec![
            "elev_above_water".into(),
            "elev_below_mountain".into(),
        ]),
    );

    // Low moisture
    spec.add(
        "moist_high",
        NodeSpec::smoothstep("moist_raw".into(), 0.30, 0.50),
    );
    spec.add("moist_low", NodeSpec::invert("moist_high".into()));

    spec.add(
        "desert_score",
        NodeSpec::mul(vec!["elev_mid".into(), "moist_low".into()]),
    );

    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("desert_score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    Kind::new("desert", spec)
}

// Mid elevation + high moisture
fn kind_forest() -> Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add(
        "elev_raw",
        NodeSpec::texture("elevation", TextureChannel::R),
    );
    spec.add(
        "moist_raw",
        NodeSpec::texture("moisture", TextureChannel::R),
    );

    spec.add(
        "elev_above_water",
        NodeSpec::smoothstep("elev_raw".into(), 0.20, 0.30),
    );
    spec.add(
        "elev_high_rise",
        NodeSpec::smoothstep("elev_raw".into(), 0.75, 0.90),
    );
    spec.add(
        "elev_below_mountain",
        NodeSpec::invert("elev_high_rise".into()),
    );
    spec.add(
        "elev_mid",
        NodeSpec::mul(vec![
            "elev_above_water".into(),
            "elev_below_mountain".into(),
        ]),
    );

    spec.add(
        "moist_high",
        NodeSpec::smoothstep("moist_raw".into(), 0.50, 0.70),
    );

    spec.add(
        "forest_score",
        NodeSpec::mul(vec!["elev_mid".into(), "moist_high".into()]),
    );

    // Slightly scale to soften competition with other biomes
    spec.add("forest_scaled", NodeSpec::scale("forest_score".into(), 0.9));

    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("forest_scaled".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    Kind::new("forest", spec)
}

// High elevation (optionally prefer lower moisture)
fn kind_mountain() -> Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add(
        "elev_raw",
        NodeSpec::texture("elevation", TextureChannel::R),
    );
    spec.add(
        "moist_raw",
        NodeSpec::texture("moisture", TextureChannel::R),
    );

    // High elevation
    spec.add(
        "elev_high",
        NodeSpec::smoothstep("elev_raw".into(), 0.70, 0.85),
    );

    // Prefer slightly lower moisture to bias "rocky" feel
    spec.add(
        "moist_high",
        NodeSpec::smoothstep("moist_raw".into(), 0.45, 0.65),
    );
    spec.add("moist_low", NodeSpec::invert("moist_high".into()));

    spec.add(
        "mountain_score",
        NodeSpec::mul(vec!["elev_high".into(), "moist_low".into()]),
    );

    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("mountain_score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    Kind::new("mountain", spec)
}

fn render(result: &RunResult) -> anyhow::Result<()> {
    let image_size = (1000, 1000);
    let domain_extent = Vec2::new(100.0, 100.0);
    let background = [236, 238, 242];

    let mut config = RenderConfig::new(image_size, domain_extent).with_background(background);

    config
        .set_kind_style(
            "water",
            KindStyle::Circle {
                color: [50, 120, 220], // blue
                radius: 3,
            },
        )
        .set_kind_style(
            "desert",
            KindStyle::Circle {
                color: [210, 170, 90], // tan
                radius: 3,
            },
        )
        .set_kind_style(
            "forest",
            KindStyle::Circle {
                color: [40, 150, 60], // green
                radius: 3,
            },
        )
        .set_kind_style(
            "mountain",
            KindStyle::Circle {
                color: [150, 150, 160], // gray
                radius: 3,
            },
        );

    let out = "grids-biome-blend.png";
    render_run_result_to_png(result, &config, out)?;
    Ok(())
}
