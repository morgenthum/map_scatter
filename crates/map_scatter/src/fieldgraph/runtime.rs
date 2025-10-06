//! Runtime for evaluating field programs and baking rasters.
//!
//! This module interprets compiled [`FieldProgram`]s,
//! sampling values on-demand via [`FieldRuntime::sample`] and optionally baking results
//! into [`Raster`]s aligned to a [`ChunkGrid`].
//! It also integrates texture inputs through [`TextureRegistry`].
use std::collections::HashMap;

use glam::Vec2;
use tracing::warn;

use crate::fieldgraph::edt::bake_edt_normalize_params;
use crate::fieldgraph::program::FieldProgram;
use crate::fieldgraph::{ChunkGrid, ChunkId, FieldId, NodeSpec, Raster, TextureRegistry};

/// Runtime for evaluating field programs, managing textures and baked rasters.
pub struct FieldRuntime<'a> {
    pub program: FieldProgram,
    pub textures: &'a TextureRegistry,
    baked_rasters: HashMap<(FieldId, ChunkId), Raster>,
}

impl<'a> FieldRuntime<'a> {
    /// Create a new field runtime with the given program and texture registry.
    pub fn new(program: FieldProgram, textures: &'a TextureRegistry) -> Self {
        Self {
            program,
            textures,
            baked_rasters: HashMap::new(),
        }
    }

    /// Sample the value of a field at a given world position within a chunk and grid.
    pub fn sample(&mut self, field: &str, p: Vec2, chunk: ChunkId, grid: &ChunkGrid) -> f32 {
        let key = (field.to_string(), chunk);

        if let Some(raster) = self.baked_rasters.get(&key) {
            return raster.sample_domain(p);
        }

        if let Some(meta) = self.program.nodes.get(field) {
            if meta.force_bake {
                self.bake_raster_if_needed(field, chunk, grid);
                if let Some(r) = self.baked_rasters.get(&key) {
                    return r.sample_domain(p);
                }
                warn!("Raster for '{}' not found after force bake.", field);
            }
        }

        self.eval_field_value(field, p, chunk, grid)
    }

    fn eval_field_value(&mut self, field: &str, p: Vec2, chunk: ChunkId, grid: &ChunkGrid) -> f32 {
        enum Op {
            Constant(f32),
            Texture(String, crate::fieldgraph::TextureChannel),
            Add(Vec<String>),
            Sub(Vec<String>),
            Scale(Option<String>, f32),
            Mul(Vec<String>),
            Min(Vec<String>),
            Max(Vec<String>),
            Invert(Option<String>),
            Clamp(Option<String>, f32, f32),
            SmoothStep(Option<String>, f32, f32),
            Pow(Option<String>, f32),
            Edt,
        }

        let op = {
            let Some(meta) = self.program.nodes.get(field) else {
                warn!("Unknown field '{}'.", field);
                return 0.0;
            };
            match &meta.spec {
                NodeSpec::Constant { params } => Op::Constant(params.value),
                NodeSpec::Texture { params } => {
                    Op::Texture(params.texture_id.clone(), params.channel)
                }
                NodeSpec::Add { inputs } => Op::Add(inputs.clone()),
                NodeSpec::Sub { inputs } => Op::Sub(inputs.clone()),
                NodeSpec::Scale { inputs, params } => {
                    Op::Scale(inputs.first().cloned(), params.factor)
                }
                NodeSpec::Mul { inputs } => Op::Mul(inputs.clone()),
                NodeSpec::Min { inputs } => Op::Min(inputs.clone()),
                NodeSpec::Max { inputs } => Op::Max(inputs.clone()),
                NodeSpec::Invert { inputs } => Op::Invert(inputs.first().cloned()),
                NodeSpec::Clamp { inputs, params } => {
                    Op::Clamp(inputs.first().cloned(), params.min, params.max)
                }
                NodeSpec::SmoothStep { inputs, params } => {
                    Op::SmoothStep(inputs.first().cloned(), params.edge0, params.edge1)
                }
                NodeSpec::Pow { inputs, params } => Op::Pow(inputs.first().cloned(), params.exp),
                NodeSpec::EdtNormalize { .. } => Op::Edt,
            }
        };

        match op {
            Op::Constant(v) => v,
            Op::Texture(id, ch) => self.textures.sample(&id, ch, p),
            Op::Add(inputs) => {
                let mut sum = 0.0;
                for id in inputs {
                    sum += self.sample(&id, p, chunk, grid);
                }
                sum
            }
            Op::Sub(inputs) => {
                if inputs.is_empty() {
                    0.0
                } else {
                    let mut iter = inputs.into_iter();
                    let mut acc = self.sample(&iter.next().unwrap(), p, chunk, grid);
                    for id in iter {
                        acc -= self.sample(&id, p, chunk, grid);
                    }
                    acc
                }
            }
            Op::Scale(input, factor) => {
                let v = if let Some(id) = input {
                    self.sample(&id, p, chunk, grid)
                } else {
                    0.0
                };
                v * factor
            }
            Op::Mul(inputs) => {
                let mut product = 1.0;
                for id in inputs {
                    product *= self.sample(&id, p, chunk, grid);
                }
                product
            }
            Op::Min(inputs) => {
                let mut min_val = f32::INFINITY;
                for id in inputs {
                    min_val = min_val.min(self.sample(&id, p, chunk, grid));
                }
                min_val
            }
            Op::Max(inputs) => {
                let mut max_val = f32::NEG_INFINITY;
                for id in inputs {
                    max_val = max_val.max(self.sample(&id, p, chunk, grid));
                }
                max_val
            }
            Op::Invert(input) => 1.0 - self.sample(input.as_deref().unwrap_or(""), p, chunk, grid),
            Op::Clamp(input, min, max) => {
                let v = self.sample(input.as_deref().unwrap_or(""), p, chunk, grid);
                v.clamp(min, max)
            }
            Op::SmoothStep(input, e0, e1) => {
                let v = self.sample(input.as_deref().unwrap_or(""), p, chunk, grid);
                smoothstep01(e0, e1, v)
            }
            Op::Pow(input, exp) => {
                let v = self.sample(input.as_deref().unwrap_or(""), p, chunk, grid);
                v.powf(exp)
            }
            Op::Edt => {
                self.bake_raster_if_needed(field, chunk, grid);
                if let Some(r) = self.baked_rasters.get(&(field.to_string(), chunk)) {
                    r.sample_domain(p)
                } else {
                    warn!("Raster for '{}' not found after baking.", field);
                    0.0
                }
            }
        }
    }

    fn bake_raster_if_needed(&mut self, field: &str, chunk: ChunkId, grid: &ChunkGrid) {
        let key = (field.to_string(), chunk);
        if self.baked_rasters.contains_key(&key) {
            return;
        }

        let Some(meta_ref) = self.program.nodes.get(field) else {
            warn!("Cannot bake unknown field '{}'.", field);
            return;
        };

        if let Some((input_id, threshold, d_max)) = {
            if let NodeSpec::EdtNormalize { inputs, params } = &meta_ref.spec {
                Some((
                    inputs.first().cloned().unwrap_or_default(),
                    params.threshold,
                    params.d_max,
                ))
            } else {
                None
            }
        } {
            let raster = bake_edt_normalize_params(self, &input_id, threshold, d_max, chunk, grid);
            self.baked_rasters.insert(key, raster);
            return;
        }

        let mut raster = Raster::new(grid.clone());
        let (tw, th) = raster.size();

        for iy in 0..th as isize {
            for ix in 0..tw as isize {
                let p = grid.index_to_world(ix, iy);
                let v = self.eval_field_value(field, p, chunk, grid);
                let idx = (iy as usize) * tw + ix as usize;
                raster.data[idx] = v;
            }
        }

        self.baked_rasters.insert(key, raster);
    }
}

fn smoothstep01(e0: f32, e1: f32, x: f32) -> f32 {
    let denom = e1 - e0;
    if denom.abs() <= f32::EPSILON {
        return if x >= e1 { 1.0 } else { 0.0 };
    }
    let t = ((x - e0) / denom).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fieldgraph::compiler::{CompileOptions, FieldGraphCompiler};
    use crate::prelude::{FieldGraphSpec, Texture, TextureChannel};

    struct ConstTexture(f32);

    impl Texture for ConstTexture {
        fn sample(&self, _channel: TextureChannel, _p: Vec2) -> f32 {
            self.0
        }
    }

    fn grid() -> ChunkGrid {
        ChunkGrid {
            origin_domain: Vec2::ZERO,
            cell_size: 1.0,
            width: 1,
            height: 1,
            halo: 0,
        }
    }

    fn approx_eq(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn runtime_evaluates_arithmetic_nodes() {
        let mut spec = FieldGraphSpec::default();
        spec.add("base", NodeSpec::constant(0.25));
        spec.add("scaled", NodeSpec::scale("base".into(), 2.0));
        spec.add("clamped", NodeSpec::clamp("scaled".into(), 0.0, 0.4));
        spec.add("inverted", NodeSpec::invert("clamped".into()));
        spec.add("powed", NodeSpec::pow("inverted".into(), 2.0));
        spec.add("smooth", NodeSpec::smoothstep("scaled".into(), 0.0, 1.0));
        spec.add("sum", NodeSpec::add(vec!["base".into(), "scaled".into()]));
        spec.add(
            "difference",
            NodeSpec::sub(vec!["scaled".into(), "base".into()]),
        );
        spec.add(
            "product",
            NodeSpec::mul(vec!["base".into(), "scaled".into()]),
        );
        spec.add(
            "minimum",
            NodeSpec::min(vec!["scaled".into(), "clamped".into()]),
        );
        spec.add(
            "maximum",
            NodeSpec::max(vec!["scaled".into(), "clamped".into()]),
        );
        spec.add(
            "texture_value",
            NodeSpec::texture("const", TextureChannel::R),
        );

        let program = FieldGraphCompiler::compile(&spec, &CompileOptions::default()).unwrap();

        let mut textures = TextureRegistry::new();
        textures.register("const", ConstTexture(0.8));

        let mut runtime = FieldRuntime::new(program, &textures);
        let grid = grid();
        let chunk = ChunkId(0, 0);

        approx_eq(runtime.sample("base", Vec2::ZERO, chunk, &grid), 0.25);
        approx_eq(runtime.sample("scaled", Vec2::ZERO, chunk, &grid), 0.5);
        approx_eq(runtime.sample("clamped", Vec2::ZERO, chunk, &grid), 0.4);
        approx_eq(runtime.sample("inverted", Vec2::ZERO, chunk, &grid), 0.6);
        approx_eq(runtime.sample("powed", Vec2::ZERO, chunk, &grid), 0.36);
        approx_eq(runtime.sample("smooth", Vec2::ZERO, chunk, &grid), 0.5);
        approx_eq(runtime.sample("sum", Vec2::ZERO, chunk, &grid), 0.75);
        approx_eq(runtime.sample("difference", Vec2::ZERO, chunk, &grid), 0.25);
        approx_eq(runtime.sample("product", Vec2::ZERO, chunk, &grid), 0.125);
        approx_eq(runtime.sample("minimum", Vec2::ZERO, chunk, &grid), 0.4);
        approx_eq(runtime.sample("maximum", Vec2::ZERO, chunk, &grid), 0.5);
        approx_eq(
            runtime.sample("texture_value", Vec2::ZERO, chunk, &grid),
            0.8,
        );
    }

    #[test]
    fn unknown_field_sample_returns_zero() {
        let program = FieldProgram {
            nodes: HashMap::new(),
            topo: Vec::new(),
        };
        let textures = TextureRegistry::new();
        let mut runtime = FieldRuntime::new(program, &textures);
        let grid = grid();
        assert_eq!(
            runtime.sample("missing", Vec2::ZERO, ChunkId(0, 0), &grid),
            0.0
        );
    }

    #[test]
    fn smoothstep_handles_degenerate_edges() {
        assert_eq!(smoothstep01(0.5, 0.5, 0.25), 0.0);
        assert_eq!(smoothstep01(0.5, 0.5, 0.5), 1.0);
        assert_eq!(smoothstep01(0.5, 0.5, 1.0), 1.0);
    }
}
