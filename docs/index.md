<div class="hero">
  <div class="hero__content">
    <img class="hero__logo" src="assets/logo.png" alt="map_scatter logo" />
    <h1>map_scatter</h1>
    <p class="hero__tagline">Rule-based object scattering for games and tools.</p>
    <p>Populate worlds with trees, rocks, props, resources, and decals using data-driven rules.</p>
    <div class="hero__actions">
      <a class="md-button md-button--primary" href="quickstart/">Getting Started</a>
      <a class="md-button" href="concepts/">Core Concepts</a>
    </div>
  </div>
</div>

## Overview

map_scatter is a Rust workspace for fast, deterministic placement in 2D domains (or 2D projections of 3D worlds). It combines field-graph evaluation with multiple sampling strategies, so you can describe where things may appear instead of writing bespoke placement code for every asset type.

## Why map_scatter

- **Rule-based placement:** author fields and gates instead of hand-coded loops.
- **Multiple sampling strategies:** blue-noise, grids, clustered, low-discrepancy, and more.
- **Deterministic results:** same inputs + seed = identical output.
- **Layered control:** order kinds, reuse overlays, and compose rules.
- **Scales to large worlds:** chunked evaluation and caching keep runs predictable.

## Typical workflow

1. Define one or more kinds with field graphs.
2. Pick a sampling strategy per layer.
3. Assemble layers into a plan.
4. Run the plan with textures, cache, and a run configuration.
5. Use placements and optional overlay masks to drive your game or tooling.

## Scope

map_scatter focuses on placement logic. It does not render, generate terrain, or manage assets for you. You supply the domain, textures, and the logic that turns placements into entities or gameplay.

## Choose your entry point

- **Core library:** use `map_scatter` directly in Rust tools or pipelines.
- **Bevy plugin:** use `bevy_map_scatter` for asset-driven authoring and ECS integration.

## Project layout

- Core library: `crates/map_scatter`
- Bevy integration: `crates/bevy_map_scatter`
- Examples: `crates/map_scatter_examples` and `crates/bevy_map_scatter_examples`

## Links

- API docs: https://docs.rs/map_scatter
- Crate: https://crates.io/crates/map_scatter
- Repository: https://github.com/morgenthum/map_scatter
