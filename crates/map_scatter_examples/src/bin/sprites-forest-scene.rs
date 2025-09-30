use glam::Vec2;
use map_scatter::prelude::*;
use map_scatter_examples::{init_tracing, render_run_result_to_png, KindStyle, RenderConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn main() {
    init_tracing();
    let domain_extent = Vec2::new(200.0, 200.0);
    let image_size = (1000, 1000);

    let river = ProceduralRiverTexture {
        domain_extent,
        river_half_width_world: 4.0,
    };
    let mut textures = TextureRegistry::new();
    textures.register("river", river);
    let rock_density = ProceduralRockDensityTexture { domain_extent };
    textures.register("rock_density", rock_density);

    let willow = willow_spec();
    let oak = oak_spec();
    let birch = birch_spec();
    let hazel = hazel_spec();
    let blackberry = blackberry_spec();
    let rock = rock_spec();
    let mushroom = mushroom_spec();
    let grass = grass_spec();

    let plan = Plan::new()
        .with_layer(
            Layer::new(
                "trees_willow",
                vec![willow],
                Box::new(PoissonDiskSampling::new(14.0)),
            )
            .with_overlay(image_size, 7),
        )
        .with_layer(
            Layer::new(
                "trees_oak",
                vec![oak],
                Box::new(PoissonDiskSampling::new(16.0)),
            )
            .with_overlay(image_size, 7),
        )
        .with_layer(
            Layer::new(
                "trees_birch",
                vec![birch],
                Box::new(PoissonDiskSampling::new(12.0)),
            )
            .with_overlay(image_size, 6),
        )
        .with_layer(
            Layer::new(
                "hazel",
                vec![hazel],
                Box::new(PoissonDiskSampling::new(8.0)),
            )
            .with_overlay(image_size, 4),
        )
        .with_layer(
            Layer::new(
                "blackberry",
                vec![blackberry],
                Box::new(PoissonDiskSampling::new(10.0)),
            )
            .with_overlay(image_size, 5),
        )
        .with_layer(
            Layer::new("rocks", vec![rock], Box::new(PoissonDiskSampling::new(8.5)))
                .with_overlay(image_size, 5),
        )
        .with_layer(
            Layer::new(
                "mushrooms",
                vec![mushroom],
                Box::new(PoissonDiskSampling::new(4.8)),
            )
            .with_overlay(image_size, 3),
        )
        .with_layer(Layer::new(
            "grass",
            vec![grass],
            Box::new(JitterGridSampling::new(1.4, 3.0)),
        ));

    let config = RunConfig::new(domain_extent)
        .with_chunk_extent(100.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(3);

    let mut cache = FieldProgramCache::new();
    let mut rng = StdRng::seed_from_u64(42);
    let mut runner = ScatterRunner::try_new(config, &textures, &mut cache).expect("valid config");
    let result = runner.run(&plan, &mut rng);

    let mut rc = RenderConfig::new(image_size, domain_extent).with_background([210, 215, 220]);
    let base = format!("{}/assets/sprites-forest-scene", env!("CARGO_MANIFEST_DIR"));
    rc.load_sprite_png("oak", format!("{base}/oak.png"))
        .expect("load oak.png");
    rc.load_sprite_png("birch", format!("{base}/birch.png"))
        .expect("load birch.png");
    rc.load_sprite_png("willow", format!("{base}/willow.png"))
        .expect("load willow.png");
    rc.load_sprite_png("rock", format!("{base}/rock.png"))
        .expect("load rock.png");
    rc.load_sprite_png("mushroom", format!("{base}/mushroom.png"))
        .expect("load mushroom.png");
    rc.load_sprite_png("grass", format!("{base}/grass.png"))
        .expect("load grass.png");
    rc.load_sprite_png("hazel", format!("{base}/hazel.png"))
        .expect("load hazel.png");
    rc.load_sprite_png("blackberry", format!("{base}/blackberry.png"))
        .expect("load blackberry.png");

    rc.set_kind_style(
        "willow",
        KindStyle::Sprite {
            sprite_id: "willow".into(),
            scale: 0.06,
        },
    );
    rc.set_kind_style(
        "oak",
        KindStyle::Sprite {
            sprite_id: "oak".into(),
            scale: 0.065,
        },
    );
    rc.set_kind_style(
        "birch",
        KindStyle::Sprite {
            sprite_id: "birch".into(),
            scale: 0.055,
        },
    );
    rc.set_kind_style(
        "rock",
        KindStyle::Sprite {
            sprite_id: "rock".into(),
            scale: 0.035,
        },
    );
    rc.set_kind_style(
        "mushroom",
        KindStyle::Sprite {
            sprite_id: "mushroom".into(),
            scale: 0.022,
        },
    );
    rc.set_kind_style(
        "grass",
        KindStyle::Sprite {
            sprite_id: "grass".into(),
            scale: 0.025,
        },
    );
    rc.set_kind_style(
        "hazel",
        KindStyle::Sprite {
            sprite_id: "hazel".into(),
            scale: 0.05,
        },
    );
    rc.set_kind_style(
        "blackberry",
        KindStyle::Sprite {
            sprite_id: "blackberry".into(),
            scale: 0.04,
        },
    );

    let out = "sprites-forest-scene.png";
    let mut placements = result.placements.clone();
    let prio = |id: &str| -> u8 {
        match id {
            "grass" => 0,
            "mushroom" => 1,
            "hazel" | "blackberry" => 2,
            "willow" | "birch" | "oak" => 3,
            "rock" => 4,
            _ => 5,
        }
    };

    placements.sort_by(|a, b| {
        let pa = prio(&a.kind_id);
        let pb = prio(&b.kind_id);
        pa.cmp(&pb).then_with(|| {
            a.position
                .y
                .partial_cmp(&b.position.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    let sorted = result.clone().with_placements(placements);

    render_run_result_to_png(&sorted, &rc, out).expect("saving PNG");
}

fn willow_spec() -> map_scatter::scatter::Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add("river_raw", NodeSpec::texture("river", TextureChannel::R));
    spec.add("outside_water", NodeSpec::invert("river_raw".into()));
    spec.add(
        "dist_norm",
        NodeSpec::edt_normalize("outside_water".into(), 0.5, 30.0),
    );
    spec.add("near", NodeSpec::invert("dist_norm".into()));
    spec.add("near_sharp", NodeSpec::pow("near".into(), 3.0));
    spec.add(
        "score",
        NodeSpec::mul(vec!["near_sharp".into(), "outside_water".into()]),
    );
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    map_scatter::scatter::Kind::new("willow", spec)
}

fn oak_spec() -> map_scatter::scatter::Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add("river_raw", NodeSpec::texture("river", TextureChannel::R));
    spec.add("outside_water", NodeSpec::invert("river_raw".into()));
    spec.add(
        "dist_norm",
        NodeSpec::edt_normalize("outside_water".into(), 0.5, 40.0),
    );

    spec.add(
        "rise_mid",
        NodeSpec::smoothstep("dist_norm".into(), 0.20, 0.65),
    );
    spec.add(
        "fall_mid_raw",
        NodeSpec::smoothstep("dist_norm".into(), 0.80, 1.00),
    );
    spec.add("fall_mid", NodeSpec::invert("fall_mid_raw".into()));
    spec.add(
        "band",
        NodeSpec::mul(vec!["rise_mid".into(), "fall_mid".into()]),
    );
    spec.add(
        "far_soft_raw",
        NodeSpec::smoothstep("dist_norm".into(), 0.85, 1.00),
    );
    spec.add("far_soft", NodeSpec::scale("far_soft_raw".into(), 0.25));
    spec.add(
        "pref",
        NodeSpec::max(vec!["band".into(), "far_soft".into()]),
    );
    spec.add(
        "score",
        NodeSpec::mul(vec!["pref".into(), "outside_water".into()]),
    );
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    map_scatter::scatter::Kind::new("oak", spec)
}

fn birch_spec() -> map_scatter::scatter::Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add("river_raw", NodeSpec::texture("river", TextureChannel::R));
    spec.add("outside_water", NodeSpec::invert("river_raw".into()));
    spec.add(
        "dist_norm",
        NodeSpec::edt_normalize("outside_water".into(), 0.5, 35.0),
    );

    spec.add("rise", NodeSpec::smoothstep("dist_norm".into(), 0.10, 0.45));
    spec.add(
        "fall_raw",
        NodeSpec::smoothstep("dist_norm".into(), 0.70, 0.95),
    );
    spec.add("fall", NodeSpec::invert("fall_raw".into()));
    spec.add("band", NodeSpec::mul(vec!["rise".into(), "fall".into()]));
    spec.add(
        "score",
        NodeSpec::mul(vec!["band".into(), "outside_water".into()]),
    );

    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    map_scatter::scatter::Kind::new("birch", spec)
}

fn hazel_spec() -> map_scatter::scatter::Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add("river_raw", NodeSpec::texture("river", TextureChannel::R));
    spec.add("outside_water", NodeSpec::invert("river_raw".into()));

    spec.add(
        "m_willow",
        NodeSpec::texture("mask_trees_willow", TextureChannel::R),
    );
    spec.add(
        "m_oak",
        NodeSpec::texture("mask_trees_oak", TextureChannel::R),
    );
    spec.add(
        "m_birch",
        NodeSpec::texture("mask_trees_birch", TextureChannel::R),
    );
    spec.add(
        "m_trees_wo",
        NodeSpec::max(vec!["m_willow".into(), "m_oak".into()]),
    );
    spec.add(
        "m_trees",
        NodeSpec::max(vec!["m_trees_wo".into(), "m_birch".into()]),
    );

    spec.add("m_inv", NodeSpec::invert("m_trees".into()));
    spec.add(
        "dist_norm",
        NodeSpec::edt_normalize("m_inv".into(), 0.5, 6.0),
    );
    spec.add("near", NodeSpec::invert("dist_norm".into()));
    spec.add(
        "away_from_trunk",
        NodeSpec::smoothstep("dist_norm".into(), 0.08, 0.18),
    );
    spec.add("near_sharp", NodeSpec::pow("near".into(), 2.0));

    spec.add(
        "score",
        NodeSpec::mul(vec![
            "near_sharp".into(),
            "away_from_trunk".into(),
            "outside_water".into(),
        ]),
    );
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    map_scatter::scatter::Kind::new("hazel", spec)
}

fn blackberry_spec() -> map_scatter::scatter::Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add("river_raw", NodeSpec::texture("river", TextureChannel::R));
    spec.add("outside_water", NodeSpec::invert("river_raw".into()));

    spec.add(
        "m_willow",
        NodeSpec::texture("mask_trees_willow", TextureChannel::R),
    );
    spec.add(
        "m_oak",
        NodeSpec::texture("mask_trees_oak", TextureChannel::R),
    );
    spec.add(
        "m_birch",
        NodeSpec::texture("mask_trees_birch", TextureChannel::R),
    );
    spec.add(
        "m_trees_wo",
        NodeSpec::max(vec!["m_willow".into(), "m_oak".into()]),
    );
    spec.add(
        "m_trees",
        NodeSpec::max(vec!["m_trees_wo".into(), "m_birch".into()]),
    );
    spec.add("m_inv", NodeSpec::invert("m_trees".into()));
    spec.add(
        "dist_norm",
        NodeSpec::edt_normalize("m_inv".into(), 0.5, 10.0),
    );

    spec.add("rise", NodeSpec::smoothstep("dist_norm".into(), 0.25, 0.50));
    spec.add(
        "fall_raw",
        NodeSpec::smoothstep("dist_norm".into(), 0.70, 0.90),
    );
    spec.add("fall", NodeSpec::invert("fall_raw".into()));
    spec.add("band", NodeSpec::mul(vec!["rise".into(), "fall".into()]));

    spec.add(
        "score",
        NodeSpec::mul(vec!["band".into(), "outside_water".into()]),
    );
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    map_scatter::scatter::Kind::new("blackberry", spec)
}

fn rock_spec() -> map_scatter::scatter::Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add("river_raw", NodeSpec::texture("river", TextureChannel::R));
    spec.add("outside_water", NodeSpec::invert("river_raw".into()));
    spec.add(
        "dist_norm",
        NodeSpec::edt_normalize("outside_water".into(), 0.5, 45.0),
    );
    spec.add(
        "far_dry",
        NodeSpec::smoothstep("dist_norm".into(), 0.60, 0.95),
    );

    spec.add(
        "m_willow",
        NodeSpec::texture("mask_trees_willow", TextureChannel::R),
    );
    spec.add(
        "m_oak",
        NodeSpec::texture("mask_trees_oak", TextureChannel::R),
    );
    spec.add(
        "m_birch",
        NodeSpec::texture("mask_trees_birch", TextureChannel::R),
    );
    spec.add(
        "m_any_wo",
        NodeSpec::max(vec!["m_willow".into(), "m_oak".into()]),
    );
    spec.add(
        "m_any",
        NodeSpec::max(vec!["m_any_wo".into(), "m_birch".into()]),
    );
    spec.add("away_from_trees", NodeSpec::invert("m_any".into()));

    spec.add(
        "rock_density",
        NodeSpec::texture("rock_density", TextureChannel::R),
    );
    spec.add(
        "rock_density_sharp",
        NodeSpec::pow("rock_density".into(), 1.6),
    );
    spec.add(
        "score",
        NodeSpec::mul(vec![
            "far_dry".into(),
            "outside_water".into(),
            "away_from_trees".into(),
            "rock_density_sharp".into(),
        ]),
    );
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    map_scatter::scatter::Kind::new("rock", spec)
}

fn mushroom_spec() -> map_scatter::scatter::Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add("river_raw", NodeSpec::texture("river", TextureChannel::R));
    spec.add("outside_water", NodeSpec::invert("river_raw".into()));

    spec.add(
        "m_oak",
        NodeSpec::texture("mask_trees_oak", TextureChannel::R),
    );
    spec.add(
        "m_birch",
        NodeSpec::texture("mask_trees_birch", TextureChannel::R),
    );
    spec.add(
        "m_any",
        NodeSpec::max(vec!["m_oak".into(), "m_birch".into()]),
    );
    spec.add("m_inv", NodeSpec::invert("m_any".into()));
    spec.add(
        "dist_norm",
        NodeSpec::edt_normalize("m_inv".into(), 0.5, 8.0),
    );
    spec.add("near", NodeSpec::invert("dist_norm".into()));
    spec.add("near_sharp", NodeSpec::pow("near".into(), 4.0));
    spec.add(
        "score",
        NodeSpec::mul(vec!["near_sharp".into(), "outside_water".into()]),
    );
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    map_scatter::scatter::Kind::new("mushroom", spec)
}

fn grass_spec() -> map_scatter::scatter::Kind {
    let mut spec = FieldGraphSpec::default();

    spec.add("river_raw", NodeSpec::texture("river", TextureChannel::R));
    spec.add("outside_water", NodeSpec::invert("river_raw".into()));

    spec.add(
        "m_willow",
        NodeSpec::texture("mask_trees_willow", TextureChannel::R),
    );
    spec.add(
        "m_oak",
        NodeSpec::texture("mask_trees_oak", TextureChannel::R),
    );
    spec.add(
        "m_birch",
        NodeSpec::texture("mask_trees_birch", TextureChannel::R),
    );
    spec.add(
        "m_rocks",
        NodeSpec::texture("mask_rocks", TextureChannel::R),
    );
    spec.add(
        "m_hazel",
        NodeSpec::texture("mask_hazel", TextureChannel::R),
    );
    spec.add(
        "m_blackberry",
        NodeSpec::texture("mask_blackberry", TextureChannel::R),
    );
    spec.add(
        "m_tree_any1",
        NodeSpec::max(vec!["m_willow".into(), "m_oak".into()]),
    );
    spec.add(
        "m_tree_any",
        NodeSpec::max(vec!["m_tree_any1".into(), "m_birch".into()]),
    );
    spec.add(
        "m_bush_any",
        NodeSpec::max(vec!["m_hazel".into(), "m_blackberry".into()]),
    );
    spec.add(
        "m_hard_any1",
        NodeSpec::max(vec!["m_tree_any".into(), "m_rocks".into()]),
    );
    spec.add(
        "m_hard_any2",
        NodeSpec::max(vec!["m_hard_any1".into(), "m_bush_any".into()]),
    );
    spec.add(
        "m_hard_any",
        NodeSpec::clamp("m_hard_any2".into(), 0.0, 1.0),
    ); // ensure in range

    spec.add("m_inv", NodeSpec::invert("m_hard_any".into()));
    spec.add(
        "dist_open",
        NodeSpec::edt_normalize("m_inv".into(), 0.5, 6.0),
    );
    spec.add(
        "away_from_river_edge_raw",
        NodeSpec::edt_normalize("outside_water".into(), 0.5, 5.0),
    );
    spec.add(
        "away_from_river_edge",
        NodeSpec::smoothstep("away_from_river_edge_raw".into(), 0.05, 0.7),
    );

    spec.add(
        "score",
        NodeSpec::mul(vec![
            "outside_water".into(),
            "dist_open".into(),
            "away_from_river_edge".into(),
        ]),
    );
    spec.add_with_semantics(
        "probability",
        NodeSpec::clamp("score".into(), 0.0, 1.0),
        FieldSemantics::Probability,
    );

    map_scatter::scatter::Kind::new("grass", spec)
}

struct ProceduralRiverTexture {
    domain_extent: Vec2,
    river_half_width_world: f32,
}

impl Texture for ProceduralRiverTexture {
    fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
        let u = ((p.x / self.domain_extent.x) + 0.5).clamp(0.0, 1.0);
        let v = ((p.y / self.domain_extent.y) + 0.5).clamp(0.0, 1.0);

        let v_center = 0.5
            + 0.22 * (std::f32::consts::TAU * u).sin()
            + 0.08 * (3.0 * std::f32::consts::TAU * u).sin();

        let width_norm = self.river_half_width_world / self.domain_extent.y;
        let dv = (v - v_center).abs();
        if dv <= width_norm {
            1.0
        } else {
            0.0
        }
    }
}

struct ProceduralRockDensityTexture {
    domain_extent: Vec2,
}

impl Texture for ProceduralRockDensityTexture {
    fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
        let u = ((p.x / self.domain_extent.x) + 0.5).clamp(0.0, 1.0);
        let v = ((p.y / self.domain_extent.y) + 0.5).clamp(0.0, 1.0);

        let t1 = (std::f32::consts::TAU * (0.90 * u + 0.10 * v + 0.13)).sin()
            * (std::f32::consts::TAU * (0.80 * v + 0.07)).cos();
        let t2 = (std::f32::consts::TAU * (0.25 * u + 0.31)).sin()
            * (std::f32::consts::TAU * (0.30 * v + 0.17)).sin();

        let val = 0.55 + 0.30 * t1 + 0.25 * t2; // ~[0,1] after clamp
        val.clamp(0.0, 1.0)
    }
}
