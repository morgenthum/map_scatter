# map_scatter

[![Build Status](https://github.com/morgenthum/map_scatter/actions/workflows/ci.yml/badge.svg)](https://github.com/morgenthum/map_scatter/actions/workflows/ci.yml)
[![Docs](https://docs.rs/map_scatter/badge.svg)](https://docs.rs/map_scatter)
[![Crate](https://img.shields.io/crates/v/map_scatter.svg)](https://crates.io/crates/map_scatter)
[![License: MIT or Apache 2.0](https://img.shields.io/badge/License-MIT%20or%20Apache2-blue.svg)](#license)

Rule-based object scattering with field-graph evaluation and flexible sampling strategies.

![logo](./logo.png)

## Overview

This repository is a Rust workspace containing:

- A fast, composable core library ([`map_scatter`](./crates/map_scatter/)) for authoring and evaluating scalar field graphs, generating candidate positions via multiple sampling strategies, and selecting placements in multi-layer plans.
- An examples crate with curated demos you can run locally.
- A future Bevy plugin ([`bevy_map_scatter`](./crates/bevy_map_scatter/)) to integrate the core library into Bevy as a plugin.

## Crates in this workspace

- Core library: `map_scatter`
  - [README.md](./crates/map_scatter/README.md)
  - [ARCHITECTURE.md](./crates/map_scatter/ARCHITECTURE.md)
- Bevy plugin: `bevy_map_scatter` (planned)
- Examples: `map_scatter_examples`
    - [README.md](./crates/map_scatter_examples/README.md)

## Quick start (core library)

Add the dependency to your own project:

````toml
[dependencies]
map_scatter = "0.2"
rand = "0.9"
glam = { version = "0.30", features = ["mint"] }
mint = "0.5"
````

Minimal usage example:

````rust
use glam::Vec2;
use rand::{SeedableRng, rngs::StdRng};
use map_scatter::prelude::*;

fn main() {
    // Define a "kind" with a trivial probability=1.0 (always placeable)
    let mut spec = FieldGraphSpec::default();
    spec.add_with_semantics(
        "probability",
        NodeSpec::constant(1.0),
        FieldSemantics::Probability,
    );
    let grass = Kind::new("grass", spec);

    // One layer with a jittered grid sampler
    let layer = Layer::new_with(
        "layer_grass",
        vec![grass],
        JitterGridSampling::new(0.35, 5.0),
    );

    // Assemble a plan
    let plan = Plan::new().with_layer(layer);

    // Run
    let mut cache = FieldProgramCache::new();
    let textures = TextureRegistry::new();
    let cfg = RunConfig::new(Vec2::new(100.0, 100.0))
        .with_chunk_extent(32.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    let mut rng = StdRng::seed_from_u64(42);
    let mut runner = ScatterRunner::new(cfg, &textures, &mut cache);
    let result = runner.run(&plan, &mut rng);

    println!("Placed {} instances.", result.placements.len());
}
````

## License

This project is dual-licensed under either:
- MIT License — ./LICENSE-MIT
- Apache License, Version 2.0 — ./LICENSE-APACHE

You may choose either license.
