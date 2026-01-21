# map_scatter

[![License: MIT or Apache 2.0](https://img.shields.io/badge/License-MIT%20or%20Apache2-blue.svg)](https://github.com/morgenthum/map_scatter#license)
[![Docs](https://docs.rs/map_scatter/badge.svg)](https://docs.rs/map_scatter)
[![Crate](https://img.shields.io/crates/v/map_scatter.svg)](https://crates.io/crates/map_scatter)
[![Build Status](https://github.com/morgenthum/map_scatter/actions/workflows/ci.yml/badge.svg)](https://github.com/morgenthum/map_scatter/actions/workflows/ci.yml)

Rule-based object scattering library with field-graph evaluation and sampling.

![logo](./logo.png)

## What is it?

**map_scatter** helps you fill 2D worlds (or a 2D projection of 3D) with lots of small things - vegetation, props, resources, decals, spawn points - quickly and reproducibly.

## How it works (short)

Three parts:
- Fields (where things may appear): numbers per position built from textures, distances, and simple operations like thresholds or masks.
- Sampling (how candidates are spread): Poisson disk/blue‑noise, jittered grid, low‑discrepancy (Halton), best‑candidate, clustered, etc.
- Layers (in what order categories are placed): later layers can avoid or reuse earlier results via overlays/masks.

Field graphs compile to a program and run in chunks with caching for stable results.

Links:
- Examples: https://github.com/morgenthum/map_scatter/blob/main/crates/map_scatter_examples/README.md
- Architecture: ./ARCHITECTURE.md



### Highlights

- Field graph authoring and compilation into an efficient program
- Chunked evaluation with raster caching for speed
- Multiple sampling strategies for candidate generation
- Per-layer selection strategies (weighted random, highest probability)
- Optional overlay generation to feed subsequent layers
- Event stream for inspection, logging, and tooling

## Use cases

Use map_scatter when you need to place many small items in a 2D domain (or a 2D projection of 3D) and still control:
- placement rules (masks, thresholds, gradients)
- density and spacing per kind
- layer ordering and interaction
- determinism and runtime cost

Examples:

1. Open-world vegetation: trees first, bushes avoid trees, herbs fill gaps; tweak textures or thresholds and rerun with the same seed.
2. City dressing: lamps along roads, planters in lit areas, clutter where space remains; overlays keep layers from colliding.
3. Dungeon population: camps in large rooms, enemies avoid camps, rare loot in dead ends with minimum spacing.

## Examples

See the [example crate](https://github.com/morgenthum/map_scatter/blob/main/crates/map_scatter_examples/README.md) for curated demos you can run locally.


## Architecture

For a high-level architecture overview, see [ARCHITECTURE.md](./ARCHITECTURE.md).

## Status

Active development; API may evolve between minor releases.

## Quick Start

Add the dependency:

```toml
[dependencies]
map_scatter = "0.4"
rand = "0.9"
glam = { version = "0.30", features = ["mint"] }
mint = "0.5"
```

Hello, scatter:

```rust
use glam::Vec2;
use rand::{SeedableRng, rngs::StdRng};

use map_scatter::prelude::*;

fn main() {
    // 1) Author a field graph for a "kind"
    //    Here, we tag a constant=1.0 as the Probability field (always placeable).
    let mut spec = FieldGraphSpec::default();
    spec.add_with_semantics(
        "probability",
        NodeSpec::constant(1.0),
        FieldSemantics::Probability,
    );
    let grass = Kind::new("grass", spec);

    // 2) Build a layer using a sampling strategy (e.g., jittered grid)
    let layer = Layer::new_with(
        "grass",
        vec![grass],
        JitterGridSampling::new(0.35, 5.0), // jitter, cell_size
    )
    // Optional: produce an overlay mask to reuse in later layers (name: "mask_grass")
    .with_overlay((256, 256), 3);

    // 3) Assemble a plan (one or more layers)
    let plan = Plan::new().with_layer(layer);

    // 4) Prepare runtime
    let cache = FieldProgramCache::new();
    let textures = TextureRegistry::new(); // Register textures as needed
    let cfg = RunConfig::new(Vec2::new(100.0, 100.0))
        .with_chunk_extent(32.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    // 5) Run
    let mut rng = StdRng::seed_from_u64(42);
    let mut runner = ScatterRunner::new(cfg, &textures, &cache);
    let result = runner.run(&plan, &mut rng);

    println!(
        "Placed {} instances (evaluated: {}, rejected: {}).",
        result.placements.len(),
        result.positions_evaluated,
        result.positions_rejected
    );
}
```

Observing events:

```rust
use rand::{SeedableRng, rngs::StdRng};
use map_scatter::prelude::*;

fn run_with_events(plan: &Plan) {
    let cache = FieldProgramCache::new();
    let textures = TextureRegistry::new();
    let cfg = RunConfig::new(glam::Vec2::new(64.0, 64.0));
    let mut rng = StdRng::seed_from_u64(7);

    let mut runner = ScatterRunner::new(cfg, &textures, &cache);

    // Capture events for inspection (warnings, per-position evaluations, overlays, etc.)
    let mut sink = VecSink::new();
    let result = runner.run_with_events(plan, &mut rng, &mut sink);

    for event in sink.into_inner() {
        match event {
            ScatterEvent::PlacementMade { placement, .. } => {
                println!("Placed '{}' at {:?}", placement.kind_id, placement.position);
            }
            ScatterEvent::Warning { context, message } => {
                eprintln!("[WARN] {context}: {message}");
            }
            _ => {}
        }
    }

    println!("Total placed: {}", result.placements.len());
}
```

## Performance Notes

- Chunked evaluation: Keeps working sets small and cache-friendly.
- Raster cell size and chunk extent control performance/quality trade-offs.
- Field programs are cached and reused per (Kind, Chunk).
- Overlays are generated only when configured on the layer.

## API Tips

- Bring common types into scope with:
  ```rust
  use map_scatter::prelude::*;
  ```
- Start simple: one kind with a constant Probability field, then introduce gates/overlays.
- Tune `RunConfig`:
  - `domain_center`: shift the evaluated window in world space (useful for streaming chunks)
  - `chunk_extent`: larger chunks reduce overhead but can increase evaluation cost
  - `raster_cell_size`: smaller cells improve accuracy at higher cost
  - `grid_halo`: extra cells for filters/EDT at chunk borders
- Overlays: bridge layers by enabling `with_overlay`, then refer to the registered texture `mask_<layer_id>` in subsequent field graphs.

## Compatibility

- 2D domains (Vec2 positions); usable for 3D by feeding height/slope textures and augmenting the 2D placement with a height component in your engine
- No engine lock-in; pair with your renderer/tooling of choice
- Integrates well with `tracing` for diagnostics
- Use `rand` RNGs; examples commonly use `StdRng`

## Benchmarks

Some micro-benchmarks are included:
```bash
cargo bench -p map_scatter
```
