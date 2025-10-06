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
- An [`map_scatter_examples`](./crates/map_scatter_examples) crate with curated demos you can run locally.
- A Bevy plugin ([`bevy_map_scatter`](./crates/bevy_map_scatter/)) to integrate the core library into Bevy as a plugin.
- A [`bevy_map_scatter_examples`](./crates/bevy_map_scatter_examples) crate with Bevy-specific examples.

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
    // 1) Author a field graph for a “kind”
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
    let mut cache = FieldProgramCache::new();
    let textures = TextureRegistry::new(); // Register textures as needed
    let cfg = RunConfig::new(Vec2::new(100.0, 100.0))
        .with_chunk_extent(32.0)
        .with_raster_cell_size(1.0)
        .with_grid_halo(2);

    // 5) Run
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
````

## License

This project is dual-licensed under either:
- [MIT License](./LICENSE-MIT) or
- [Apache License, Version 2.0](./LICENSE-APACHE)
