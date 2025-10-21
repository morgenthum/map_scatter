# map_scatter

[![Build Status](https://github.com/morgenthum/map_scatter/actions/workflows/ci.yml/badge.svg)](https://github.com/morgenthum/map_scatter/actions/workflows/ci.yml)
[![Docs](https://docs.rs/map_scatter/badge.svg)](https://docs.rs/map_scatter)
[![Crate](https://img.shields.io/crates/v/map_scatter.svg)](https://crates.io/crates/map_scatter)
[![License: MIT or Apache 2.0](https://img.shields.io/badge/License-MIT%20or%20Apache2-blue.svg)](#license)

Rule‑based object scattering for games and tools with clear rules, multiple distribution styles, and reproducible results. Use it to populate worlds with trees, rocks, props, resources, and decals.

![logo](./logo.png)

## What is this?

This repository is a Rust workspace with:

- Core library: [`map_scatter`](./crates/map_scatter/) - fast, composable engine for rules, sampling, and layering.
- Bevy plugin: [`bevy_map_scatter`](./crates/bevy_map_scatter/) - Bevy integration (Assets, ECS, async).
- Examples: [`map_scatter_examples`](./crates/map_scatter_examples) and [`bevy_map_scatter_examples`](./crates/bevy_map_scatter_examples).

## Where to start

- Building your own engine/tools? Start with the core crate: [`crates/map_scatter`](./crates/map_scatter/).
- Using Bevy? Start with the plugin: [`crates/bevy_map_scatter`](./crates/bevy_map_scatter/).

The crate READMEs include:
- Practical use cases
- A short "how it works" (fields => sampling => layering)
- Quick Start and links to runnable examples

For architecture details, see [`crates/map_scatter/ARCHITECTURE.md`](./crates/map_scatter/ARCHITECTURE.md).

## Why use it

- Data‑driven placement instead of bespoke loops
- Mix distribution styles (blue‑noise, grids, clustered, low‑discrepancy)
- Deterministic, chunked evaluation for performance and reproducibility

## License

This project is dual-licensed under either:
- [MIT License](./LICENSE-MIT) or
- [Apache License, Version 2.0](./LICENSE-APACHE)
