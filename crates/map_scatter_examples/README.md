# map_scatter_examples

Run any example with `cargo run -p map_scatter_examples --bin <name>` (add `--release` for faster renders).

## Table of Contents

- [Scenes](#scenes)
- [Fields](#fields)
- [Grids](#grids)
- [Samplers](#samplers)
- [Textures](#textures)

## Scenes

### Scene - Sprites Forest Scene
Source: [src/bin/sprites-forest-scene.rs](src/bin/sprites-forest-scene.rs)

Full sprite-based forest composition with layered trees, shrubs, rocks, mushrooms, and grass.

![Sprites forest scene](images/sprites-forest-scene.png)

## Fields

### Fields - Distance Field Edge Attraction
Source: [src/bin/fields-distance-field-edge-attraction.rs](src/bin/fields-distance-field-edge-attraction.rs)

Bright edges from a distance field texture pull placements toward white geometry.

![Distance field edge attraction](images/fields-distance-field-edge-attraction.png)

### Fields - Gate Exclusion Zone
Source: [src/bin/fields-gate-exclusion-zone.rs](src/bin/fields-gate-exclusion-zone.rs)

Procedural disk gate blocks a Poisson scatter inside the forbidden region.

![Gate exclusion zone](images/fields-gate-exclusion-zone.png)

### Fields - Probability Linear Gradient
Source: [src/bin/fields-probability-linear-gradient.rs](src/bin/fields-probability-linear-gradient.rs)

A linear texture gradient biases sampling density from left to right.

![Linear gradient probability](images/fields-probability-linear-gradient.png)

## Grids

### Grids - Bilinear vs Nearest
Source: [src/bin/grids-bilinear-vs-nearest.rs](src/bin/grids-bilinear-vs-nearest.rs)

Shows how the same raster grid looks with nearest neighbor compared to bilinear sampling.

#### Nearest sampling
![Nearest sampling](images/grids-bilinear-vs-nearest-nearest.png)

#### Bilinear sampling
![Bilinear sampling](images/grids-bilinear-vs-nearest-bilinear.png)

### Grids - Biome Blend
Source: [src/bin/grids-biome-blend.rs](src/bin/grids-biome-blend.rs)

Blends water, desert, forest, and mountain kinds from synthetic elevation and moisture grids.

![Biome blend](images/grids-biome-blend.png)

### Grids - Radial Probability
Source: [src/bin/grids-radial-probability.rs](src/bin/grids-radial-probability.rs)

Radial falloff grid keeps more placements near the center of the domain.

![Radial probability](images/grids-radial-probability.png)

### Grids - Road Exclusion Mask
Source: [src/bin/grids-road-exclusion-mask.rs](src/bin/grids-road-exclusion-mask.rs)

Procedural distance-to-road mask excludes placements along a polyline corridor.

![Road exclusion mask](images/grids-road-exclusion-mask.png)

## Samplers

### Samplers - Best Candidate k
Source: [src/bin/samplers-best-candidate-k.rs](src/bin/samplers-best-candidate-k.rs)

Best-candidate sampling trade-off between a low and high candidate count.

#### k = 8
![Best candidate k=8](images/samplers-best-candidate-k-low.png)

#### k = 32
![Best candidate k=32](images/samplers-best-candidate-k-high.png)

### Samplers - Clustered Thomas vs Neyman-Scott
Source: [src/bin/samplers-clustered-thomas-vs-neyman-scott.rs](src/bin/samplers-clustered-thomas-vs-neyman-scott.rs)

Compares clustered processes with Gaussian (Thomas) and uniform disk (Neyman-Scott) kernels.

#### Thomas process
![Thomas process](images/samplers-clustered-thomas.png)

#### Neyman-Scott process
![Neyman-Scott process](images/samplers-clustered-neyman-scott.png)

### Samplers - Fibonacci Lattice Basic
Source: [src/bin/samplers-fibonacci-lattice-basic.rs](src/bin/samplers-fibonacci-lattice-basic.rs)

Fibonacci lattice scatter with Cranley-Patterson rotation for uniform coverage.

![Fibonacci lattice](images/samplers-fibonacci-lattice-basic.png)

### Samplers - Grid vs Jitter
Source: [src/bin/samplers-grid-vs-jitter.rs](src/bin/samplers-grid-vs-jitter.rs)

Baseline grid sampling against a fully jittered grid with the same cell size.

#### Pure grid
![Pure grid](images/samplers-grid-vs-jitter-grid.png)

#### Fully jittered
![Fully jittered](images/samplers-grid-vs-jitter-jittered.png)

### Samplers - Halton vs Uniform
Source: [src/bin/samplers-halton-vs-uniform.rs](src/bin/samplers-halton-vs-uniform.rs)

Low-discrepancy Halton points compared to independent uniform samples.

#### Halton sequence
![Halton sequence](images/samplers-halton-vs-uniform-halton.png)

#### Uniform random
![Uniform random](images/samplers-halton-vs-uniform-uniform.png)

### Samplers - Hex vs Square Grid
Source: [src/bin/samplers-hex-vs-square-grid.rs](src/bin/samplers-hex-vs-square-grid.rs)

Hexagonal lattice sampling beside a square jittered lattice at similar density.

#### Hex lattice
![Hex lattice](images/samplers-hex-vs-square-grid-hex.png)

#### Square lattice
![Square lattice](images/samplers-hex-vs-square-grid-square.png)

### Samplers - Poisson Basic
Source: [src/bin/samplers-poisson-basic.rs](src/bin/samplers-poisson-basic.rs)

Classic Poisson-disk sampling with a fixed minimum spacing.

![Poisson disk basic](images/samplers-poisson-basic.png)

### Samplers - Poisson vs Jitter Grid
Source: [src/bin/samplers-poisson-vs-jitter-grid.rs](src/bin/samplers-poisson-vs-jitter-grid.rs)

Two-layer scene mixing jittered grass coverage with Poisson distributed trees.

![Poisson vs jitter grid](images/samplers-poisson-vs-jitter-grid.png)

### Samplers - Stratified Multi-Jitter Basic
Source: [src/bin/samplers-stratified-multi-jitter-basic.rs](src/bin/samplers-stratified-multi-jitter-basic.rs)

Stratified multi-jitter sampler producing low-variance blue noise.

![Stratified multi-jitter](images/samplers-stratified-multi-jitter-basic.png)

### Samplers - Uniform Random Basic
Source: [src/bin/samplers-uniform-random-basic.rs](src/bin/samplers-uniform-random-basic.rs)

Simple uniform random baseline for comparison against structured samplers.

![Uniform random basic](images/samplers-uniform-random-basic.png)

## Textures

### Textures - Alpha Overlay
Source: [src/bin/textures-alpha-overlay.rs](src/bin/textures-alpha-overlay.rs)

Demonstrates combining a base pattern with a radial alpha overlay using field graph ops.

![Alpha overlay](images/textures-alpha-overlay.png)

### Textures - Channel Split
Source: [src/bin/textures-channel-split.rs](src/bin/textures-channel-split.rs)

Two kinds share one texture by sampling different color channels.

![Channel split](images/textures-channel-split.png)

### Textures - Mask Disk Basic
Source: [src/bin/textures-mask-disk-basic.rs](src/bin/textures-mask-disk-basic.rs)

Disk mask texture gates placements so only the interior is populated.

![Mask disk basic](images/textures-mask-disk-basic.png)

### Textures - Splatmap Masked Scatter
Source: [src/bin/textures-splatmap-masked-scatter.rs](src/bin/textures-splatmap-masked-scatter.rs)

Splatmap-driven blend of two kinds with a separate exclusion mask.

![Splatmap masked scatter](images/textures-splatmap-masked-scatter.png)