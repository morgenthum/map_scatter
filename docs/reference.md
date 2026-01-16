# Reference

This page highlights the most important entry points, configuration knobs, and compatibility notes. For full API details, see the docs.rs pages linked below.

## Core API entry points

- `FieldGraphSpec`, `NodeSpec`, `FieldSemantics`: author field graphs.
- `Kind`, `Layer`, `Plan`: structure your scatter plan.
- `PositionSampling`: trait implemented by sampling strategies.
- `RunConfig`, `ScatterRunner`, `RunResult`: execute plans and inspect results.
- `TextureRegistry`, `Texture`, `TextureChannel`: provide external data to fields.
- `FieldProgramCache`: reuse compiled programs across runs.

## RunConfig tuning

- `domain_extent`: size of the evaluated area in world units.
- `domain_center`: world-space center of the domain (useful for streaming).
- `chunk_extent`: chunk size in world units; larger chunks reduce overhead but increase per-chunk work.
- `raster_cell_size`: resolution of field sampling; smaller values increase accuracy at higher cost.
- `grid_halo`: extra cells around chunks for filters and EDT.

## Events and diagnostics

- Use `ScatterRunner::run_with_events` with `VecSink` or `FnSink` to observe placements, warnings, overlays, and per-position evaluations.
- In Bevy, listen to `ScatterMessage` for streaming diagnostics or pipeline telemetry.

## Feature flags

- `map_scatter`:
  - `serde` enables serialization of field graph specs and textures.
- `bevy_map_scatter`:
  - `serde` and `ron` are enabled by default for `*.scatter` assets.

## Version compatibility

| bevy_map_scatter | map_scatter | bevy |
| --- | --- | --- |
| 0.3 | 0.3 | 0.18 |

## Links

- Core API: https://docs.rs/map_scatter
- Bevy API: https://docs.rs/bevy_map_scatter
- Examples: https://github.com/morgenthum/map_scatter/tree/main/crates/map_scatter_examples
