# Bevy Integration

`bevy_map_scatter` wraps the core crate with asset loading, async execution, and ECS-friendly results.

## Install

```toml
[dependencies]
bevy = "0.18"
bevy_map_scatter = "0.4"
```

The plugin enables `serde` and `ron` by default so you can author `*.scatter` assets.

## Add the plugin

```rust
use bevy::prelude::*;
use bevy_map_scatter::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MapScatterPlugin)
        .run();
}
```

## Author a scatter plan (RON)

Plans are assets. Create a file like `assets/simple.scatter`:

```ron
(
  layers: [
    (
      id: "trees",
      kinds: [
        (
          id: "tree",
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
      sampling: PoissonDisk(
        radius: 2.5,
      ),
      selection_strategy: WeightedRandom,
    ),
  ],
)
```

## Trigger a run

Load the plan and send a `ScatterRequest` once it is ready:

```rust
use bevy::prelude::*;
use bevy_map_scatter::prelude::*;

#[derive(Resource, Default)]
struct PlanHandle(Handle<ScatterPlanAsset>);

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

    let domain = Vec2::new(100.0, 100.0);
    let config = RunConfig::new(domain)
        .with_chunk_extent(32.0)
        .with_raster_cell_size(1.0);

    let entity = commands.spawn_empty().id();
    commands.trigger(ScatterRequest::new(entity, handle.0.clone(), config, 123));

    *once = true;
}

fn on_finished(finished: On<ScatterFinished>) {
    info!("Placed {} instances", finished.result.placements.len());
}
```

## Register textures

To use Bevy `Image` assets inside field graphs, snapshot them to `ImageTexture` and register them in the shared registry. The registry is an `Arc<TextureRegistry>`, so use `Arc::make_mut` to register.

```rust
use std::sync::Arc;
use bevy::prelude::*;
use bevy_map_scatter::prelude::*;

#[derive(Resource)]
struct HeightmapHandle(Handle<Image>);

fn register_textures(
    mut registry: ResMut<ScatterTextureRegistry>,
    images: Res<Assets<Image>>,
    handle: Res<HeightmapHandle>,
) {
    let Some(image) = images.get(&handle.0) else {
        return;
    };

    let Some(texture) = ImageTexture::from_image(image, Vec2::new(100.0, 100.0)) else {
        return;
    };

    Arc::make_mut(&mut registry.0).register("heightmap", texture);
}
```

If the source image changes, create a new `ImageTexture` snapshot and re-register it.

## Streaming (optional)

For large or moving worlds, enable chunked streaming around an anchor entity:

- Add `MapScatterStreamingPlugin`.
- Attach `ScatterStreamSettings` to an entity that moves through the world.
- Listen for `ScatterStreamPlacement` components on spawned entities.

## Tips

- Use deterministic seeds during development to compare changes.
- Keep domain sizes consistent with texture mappings.
- Use overlay masks to prevent overlap between layers.
- Inspect `ScatterMessage` events for diagnostics.

## Examples

- `cargo run -p bevy_map_scatter_examples --bin quick-start`
- `cargo run -p bevy_map_scatter_examples --bin streaming-minimal`
