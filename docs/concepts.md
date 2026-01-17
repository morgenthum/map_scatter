# Core Concepts

This page describes the building blocks of map_scatter and how they fit together.

## Domain and coordinates

Scatter runs operate on a 2D domain described by `RunConfig::domain_extent` and `RunConfig::domain_center`. Positions are `Vec2` values in world units. If you are working in 3D, project your world into a 2D plane and use textures or fields for height/slope constraints.

## Kinds

A **kind** represents one category of placement (for example: grass, rocks, props). Each kind has an id and a field graph that determines where it is allowed and how likely it is to appear.

## Field graphs and semantics

A field graph is a small dataflow graph of `NodeSpec` nodes. Each node computes a value per position, often based on textures or other fields.

Two semantics drive evaluation:

- **Gate:** a field tagged as `Gate` must be positive for a placement to be allowed.
- **Probability:** a field tagged as `Probability` becomes the placement weight in `[0, 1]`.

If a kind has no probability field, a default weight is used. Gates are evaluated first; if any gate is not positive, the position is rejected.

## Sampling

Sampling strategies generate candidate positions across the domain. You can choose from multiple styles such as grid-based, blue-noise/Poisson, clustered, and low-discrepancy samplers. Sampling is independent from field evaluation, which means you can swap distribution styles without rewriting your field logic.

## Layers and plans

A **layer** combines a sampling strategy with one or more kinds. Layers are ordered in a **plan**, and each layer can optionally emit an overlay mask for later layers to read.

When multiple kinds are allowed at a position, the selection strategy decides what gets placed:

- `WeightedRandom` (default)
- `HighestProbability`

## Textures and overlays

Textures provide external data to field graphs via the `TextureRegistry`. Overlays are generated masks from previous layers and are registered as textures named `mask_<layer_id>`.

In Bevy, use `ImageTexture` to snapshot `Image` assets into CPU-side textures that can be registered with the `ScatterTextureRegistry` resource.

## Determinism and streaming

Determinism comes from combining a fixed RNG seed with a stable plan and input textures. Chunked evaluation keeps memory usage predictable and supports streaming around a moving origin by shifting `domain_center`.

## Events and observability

Scatter runs can emit `ScatterEvent` values (start, finish, per-position evaluation, overlays, warnings). Use `VecSink`, `FnSink`, or custom sinks to collect data for logs, tools, or debugging.
