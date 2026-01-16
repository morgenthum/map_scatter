# Getting Started

This page gets you from zero to a working scatter run. Choose the core library if you want a lightweight Rust API, or the Bevy plugin if you want asset-driven authoring and ECS integration.

## Prerequisites

- Rust 1.89 (see `rust-toolchain.toml`)
- Basic familiarity with Rust and Cargo

## Install

=== "Core library"
    ```toml
    [dependencies]
    map_scatter = "0.3"
    rand = "0.9"
    glam = { version = "0.30", features = ["mint"] }
    mint = "0.5"
    ```

    Optional features:

    - `map_scatter = { version = "0.3", features = ["serde"] }` to enable `serde` support.

=== "Bevy plugin"
    ```toml
    [dependencies]
    bevy = "0.18"
    bevy_map_scatter = "0.3"
    ```

    Optional:

    - Add `map_scatter = "0.3"` if you want core types directly.

## Hello, scatter

=== "Core library"
    ```rust
    use glam::Vec2;
    use rand::{rngs::StdRng, SeedableRng};

    use map_scatter::prelude::*;

    fn main() {
        // 1) Author a field graph for a "kind".
        let mut spec = FieldGraphSpec::default();
        spec.add_with_semantics(
            "probability",
            NodeSpec::constant(1.0),
            FieldSemantics::Probability,
        );
        let grass = Kind::new("grass", spec);

        // 2) Build a layer using a sampling strategy (jittered grid here).
        let layer = Layer::new_with(
            "grass",
            vec![grass],
            JitterGridSampling::new(0.35, 5.0),
        )
        // Optional: produce an overlay mask to reuse in later layers.
        .with_overlay((256, 256), 3);

        // 3) Assemble a plan (one or more layers).
        let plan = Plan::new().with_layer(layer);

        // 4) Prepare runtime.
        let mut cache = FieldProgramCache::new();
        let textures = TextureRegistry::new();
        let cfg = RunConfig::new(Vec2::new(100.0, 100.0))
            .with_chunk_extent(32.0)
            .with_raster_cell_size(1.0)
            .with_grid_halo(2);

        // 5) Run.
        let mut rng = StdRng::seed_from_u64(42);
        let mut runner = ScatterRunner::new(cfg, &textures, &mut cache);
        let result = runner.run(&plan, &mut rng);

        println!(
            "Placed {} instances (evaluated: {}, rejected: {}).",
            result.placements.len(),
            result.positions_evaluated,
            result.positions_rejected
        );
    }
    ```

=== "Bevy plugin"
    Create a scatter plan in `assets/simple.scatter`:

    ```ron
    (
      layers: [
        (
          id: "dots",
          kinds: [
            (
              id: "dots",
              spec: (
                nodes: {
                  "probability": Constant(
                    params: ConstantParams(value: 1.0),
                  ),
                },
                semantics: {
                  "probability": Probability,
                },
              ),
            ),
          ],
          sampling: JitterGrid(
            jitter: 1.0,
            cell_size: 1.0,
          ),
          selection_strategy: WeightedRandom,
        ),
      ],
    )
    ```

    Trigger a scatter run once the asset is loaded:

    ```rust
    use bevy::prelude::*;
    use bevy_map_scatter::prelude::*;

    #[derive(Resource, Default)]
    struct PlanHandle(Handle<ScatterPlanAsset>);

    fn main() {
        App::new()
            .init_resource::<PlanHandle>()
            .add_plugins(DefaultPlugins)
            .add_plugins(MapScatterPlugin)
            .add_systems(Startup, load_plan)
            .add_systems(Update, trigger_request)
            .add_observer(log_finished)
            .run();
    }

    fn load_plan(mut handle: ResMut<PlanHandle>, assets: Res<AssetServer>) {
        handle.0 = assets.load("simple.scatter");
    }

    fn trigger_request(
        mut commands: Commands,
        mut once: Local<bool>,
        handle: Res<PlanHandle>,
        assets: Res<Assets<ScatterPlanAsset>>,
    ) {
        if *once || assets.get(&handle.0).is_none() {
            return;
        }

        let domain = Vec2::new(10.0, 10.0);
        let config = RunConfig::new(domain)
            .with_chunk_extent(domain.x)
            .with_raster_cell_size(1.0);

        let entity = commands.spawn_empty().id();
        commands.trigger(ScatterRequest::new(entity, handle.0.clone(), config, 42));

        *once = true;
    }

    fn log_finished(finished: On<ScatterFinished>, mut commands: Commands) {
        info!(
            "Scatter run {} finished: placements={} evaluated={} rejected={}",
            finished.entity,
            finished.result.placements.len(),
            finished.result.positions_evaluated,
            finished.result.positions_rejected
        );

        commands.entity(finished.entity).despawn();
    }
    ```

## Run examples

- Core library: `cargo run -p map_scatter_examples --bin samplers-poisson-basic`
- Bevy: `cargo run -p bevy_map_scatter_examples --bin quick-start`

See `crates/map_scatter_examples/src/bin` and `crates/bevy_map_scatter_examples/src/bin` for the full list.

## Next steps

- Read the [Concepts](concepts.md) page to understand field graphs and layers.
- Follow the [Bevy Integration](bevy.md) guide for asset workflows and streaming.
- Review the [Architecture](architecture.md) overview for a deeper model of the pipeline.
