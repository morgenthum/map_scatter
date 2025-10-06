# bevy_map_scatter

[![License: MIT or Apache 2.0](https://img.shields.io/badge/License-MIT%20or%20Apache2-blue.svg)](https://github.com/morgenthum/map_scatter#license)
[![Docs](https://docs.rs/bevy_map_scatter/badge.svg)](https://docs.rs/bevy_map_scatter)
[![Crate](https://img.shields.io/crates/v/bevy_map_scatter.svg)](https://crates.io/crates/bevy_map_scatter)
[![Build Status](https://github.com/morgenthum/map_scatter/actions/workflows/ci.yml/badge.svg)](https://github.com/morgenthum/map_scatter/actions/workflows/ci.yml)

Bevy plugin that integrates the `map_scatter` core crate for object scattering with field-graph evaluation and sampling.

![logo](./logo.png)

## Overview

**bevy_map_scatter** wires the [**map_scatter**](/crates/map_scatter/) runtime into Bevy in an ECS- and editor-friendly way:
- Asset-based authoring of scatter plans (RON): load `*.scatter` files via `AssetServer`.
- Texture integration: snapshot Bevy `Image`s to CPU textures with configurable domain mapping.
- Asynchronous execution: runs scatter jobs on `AsyncComputeTaskPool`.
- Streaming diagnostics: forward core `ScatterEvent`s as Bevy messages (`ScatterMessage`).

## Examples

See the [example crate](/crates/bevy_map_scatter_examples) for curated demos you can run locally.

## Quick Start

Add the crates to your Bevy application:

```toml
# Cargo.toml
[dependencies]
bevy = "0.17"
bevy_map_scatter = "0.2"
map_scatter = "0.2"
```

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

Use the plugin and trigger a single scatter run once the asset is ready:

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

/// Loads the scatter plan asset on startup.
fn load_plan(mut handle: ResMut<PlanHandle>, assets: Res<AssetServer>) {
    handle.0 = assets.load("simple.scatter");
}

/// Triggers a scatter request once the plan asset is loaded.
fn trigger_request(
    mut commands: Commands,
    mut once: Local<bool>,
    handle: Res<PlanHandle>,
    assets: Res<Assets<ScatterPlanAsset>>,
) {
    // Only run once.
    if *once {
        return;
    }
    // Wait until the asset is loaded.
    if assets.get(&handle.0).is_none() {
        return;
    }

    // The domain size for scattering.
    let domain = Vec2::new(10.0, 10.0);

    // Create run configuration and seed for (deterministic) randomness.
    let config = RunConfig::new(domain)
        .with_chunk_extent(domain.x)
        .with_raster_cell_size(1.0);

    // Spawn an entity to track the request.
    // In real applications you might want to add your own components here,
    // or use an existing entity.
    let entity = commands.spawn_empty().id();

    // Trigger the scatter run.
    commands.trigger(ScatterRequest::new(entity, handle.0.clone(), config, 42));

    // Mark as done.
    *once = true;
}

/// Observes the `EntityEvent` when a scatter run has finished.
fn log_finished(finished: On<ScatterFinished>, mut commands: Commands) {
    info!(
        "Scatter run {} finished: placements={} evaluated={} rejected={}",
        finished.entity,
        finished.result.placements.len(),
        finished.result.positions_evaluated,
        finished.result.positions_rejected
    );

    // Clean up the entity used for the request.
    // In real applications you might want to keep it,
    // depending on your use case.
    commands.entity(finished.entity).despawn();
}
```

Run the application with `cargo run`. Once the scatter job completes you will see a summary in the log and can continue with your own placement logic.

## Compatibility

| `bevy_map_scatter` | `map_scatter` | `bevy` |
| ------------------ | ------------- | ------ |
| `0.2`              | `0.2`         | `0.17` |
