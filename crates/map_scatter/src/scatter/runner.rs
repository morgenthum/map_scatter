//! High-level runner for executing scatter plans across layers and positions.
use std::collections::HashMap;
use std::sync::Arc;

use glam::Vec2;
use rand::RngCore;
use tracing::{info, warn};

use crate::error::{Error, Result};
use crate::fieldgraph::cache::FieldProgramCache;
use crate::fieldgraph::compiler::CompileOptions;
use crate::fieldgraph::program::FieldProgram;
use crate::fieldgraph::runtime::FieldRuntime;
use crate::fieldgraph::{ChunkId, TextureRegistry};
use crate::scatter::evaluator::KindEvaluation;
use crate::scatter::events::{EventSink, OverlaySummary, ScatterEvent, ScatterEventKind};
use crate::scatter::overlay::{build_overlay_mask_from_positions_in_domain, OverlayTexture};
use crate::scatter::plan::{Layer, Plan, SelectionStrategy};
use crate::scatter::selection::{pick_highest_probability, pick_weighted_random};
use crate::scatter::{chunk, Kind, KindId, DEFAULT_PROBABILITY_WHEN_MISSING};

/// Represents a placed instance of a kind at a specific position.
#[derive(Debug, Clone)]
pub struct Placement {
    /// Kind identifier for this placement.
    pub kind_id: KindId,
    /// World/domain position of the placement.
    pub position: Vec2,
}

/// Configuration for running a scatter plan.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Size of the evaluated domain in world units.
    pub domain_extent: Vec2,
    /// World-space center of the evaluated domain.
    pub domain_center: Vec2,
    /// Chunk size used for evaluation in world units.
    pub chunk_extent: f32,
    /// Raster cell size used for field sampling in world units.
    pub raster_cell_size: f32,
    /// Extra halo cells around each chunk for filters and EDT.
    pub grid_halo: usize,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            domain_extent: Vec2::new(0.0, 0.0),
            domain_center: Vec2::ZERO,
            chunk_extent: 100.0,
            raster_cell_size: 1.0,
            grid_halo: 2,
        }
    }
}

impl RunConfig {
    /// Creates a new [`RunConfig`] with the specified domain extent.
    pub fn new(domain_extent: Vec2) -> Self {
        Self {
            domain_extent,
            domain_center: Vec2::ZERO,
            ..Default::default()
        }
    }

    /// Sets the chunk extent.
    pub fn with_chunk_extent(mut self, chunk_extent: f32) -> Self {
        self.chunk_extent = chunk_extent;
        self
    }

    /// Sets the domain center in world coordinates.
    pub fn with_domain_center(mut self, domain_center: Vec2) -> Self {
        self.domain_center = domain_center;
        self
    }

    /// Sets the raster cell size.
    pub fn with_raster_cell_size(mut self, raster_cell_size: f32) -> Self {
        self.raster_cell_size = raster_cell_size;
        self
    }

    /// Sets the grid halo size.
    pub fn with_grid_halo(mut self, grid_halo: usize) -> Self {
        self.grid_halo = grid_halo;
        self
    }

    /// Validates the configuration, returning an error if invalid.
    pub fn validate(&self) -> Result<()> {
        if self.domain_extent.x <= 0.0 || self.domain_extent.y <= 0.0 {
            return Err(Error::InvalidConfig(
                "domain_extent must be > 0 in both components".into(),
            ));
        }
        if self.chunk_extent <= 0.0 {
            return Err(Error::InvalidConfig("chunk_extent must be > 0".into()));
        }
        if self.raster_cell_size <= 0.0 {
            return Err(Error::InvalidConfig("raster_cell_size must be > 0".into()));
        }

        Ok(())
    }
}

/// Result of running a scatter plan or layer.
#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct RunResult {
    /// Placements produced by the run.
    pub placements: Vec<Placement>,
    /// Total candidate positions evaluated.
    pub positions_evaluated: usize,
    /// Total candidate positions rejected.
    pub positions_rejected: usize,
}

impl RunResult {
    /// Creates a new empty [`RunResult`].
    pub fn new() -> Self {
        Self {
            placements: Vec::new(),
            positions_evaluated: 0,
            positions_rejected: 0,
        }
    }

    /// Sets the placements and returns a new instance.
    pub fn with_placements(mut self, placements: Vec<Placement>) -> Self {
        self.placements = placements;
        self
    }
}

pub struct ScatterRunner<'a> {
    /// Run configuration applied to this runner.
    pub config: RunConfig,
    /// Shared texture registry used during evaluation.
    pub base_textures: &'a TextureRegistry,
    /// Program cache used to reuse compiled field graphs.
    pub cache: &'a FieldProgramCache,
}

impl<'a> ScatterRunner<'a> {
    pub fn try_new(
        config: RunConfig,
        base_textures: &'a TextureRegistry,
        cache: &'a FieldProgramCache,
    ) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            base_textures,
            cache,
        })
    }

    pub fn new(
        config: RunConfig,
        base_textures: &'a TextureRegistry,
        cache: &'a FieldProgramCache,
    ) -> Self {
        debug_assert!(
            config.domain_extent.x > 0.0 && config.domain_extent.y > 0.0,
            "domain_extent must be > 0 in both components"
        );
        debug_assert!(config.chunk_extent > 0.0, "chunk_extent must be > 0");
        debug_assert!(
            config.raster_cell_size > 0.0,
            "raster_cell_size must be > 0"
        );

        Self {
            config,
            base_textures,
            cache,
        }
    }

    /// Runs the given plan, returning the result.
    pub fn run(&mut self, plan: &Plan, rng: &mut impl RngCore) -> RunResult {
        run_plan(
            plan,
            &self.config,
            self.base_textures,
            self.cache,
            rng,
            None,
        )
    }

    pub fn run_with_events(
        &mut self,
        plan: &Plan,
        rng: &mut impl RngCore,
        sink: &mut dyn EventSink,
    ) -> RunResult {
        run_plan(
            plan,
            &self.config,
            self.base_textures,
            self.cache,
            rng,
            Some(sink),
        )
    }

    pub fn run_layer(
        &mut self,
        layer: &Layer,
        overlays: &HashMap<String, Arc<OverlayTexture>>,
        rng: &mut impl RngCore,
    ) -> (RunResult, Option<(String, Arc<OverlayTexture>)>) {
        run_layer(
            layer,
            &self.config,
            self.base_textures,
            overlays,
            self.cache,
            rng,
            None,
        )
    }

    pub fn run_layer_with_events(
        &mut self,
        layer: &Layer,
        overlays: &HashMap<String, Arc<OverlayTexture>>,
        rng: &mut impl RngCore,
        sink: &mut dyn EventSink,
    ) -> (RunResult, Option<(String, Arc<OverlayTexture>)>) {
        run_layer(
            layer,
            &self.config,
            self.base_textures,
            overlays,
            self.cache,
            rng,
            Some(sink),
        )
    }
}

pub fn run_layer<R: RngCore>(
    layer: &Layer,
    config: &RunConfig,
    base_textures: &TextureRegistry,
    overlays: &HashMap<String, Arc<OverlayTexture>>,
    cache: &FieldProgramCache,
    rng: &mut R,
    sink: Option<&mut dyn EventSink>,
) -> (RunResult, Option<(String, Arc<OverlayTexture>)>) {
    let ctx = LayerExecContext {
        config,
        base_textures,
        overlays,
    };
    if let Some(s) = sink {
        run_layer_with_events_internal(layer, &ctx, cache, rng, s, 0)
    } else {
        run_layer_with_events_internal(layer, &ctx, cache, rng, &mut (), 0)
    }
}

pub fn run_layer_with_events<R: RngCore>(
    layer: &Layer,
    config: &RunConfig,
    base_textures: &TextureRegistry,
    overlays: &HashMap<String, Arc<OverlayTexture>>,
    cache: &FieldProgramCache,
    rng: &mut R,
    sink: &mut dyn EventSink,
) -> (RunResult, Option<(String, Arc<OverlayTexture>)>) {
    let ctx = LayerExecContext {
        config,
        base_textures,
        overlays,
    };
    run_layer_with_events_internal(layer, &ctx, cache, rng, sink, 0)
}

struct LayerExecContext<'a> {
    config: &'a RunConfig,
    base_textures: &'a TextureRegistry,
    overlays: &'a HashMap<String, Arc<OverlayTexture>>,
}

fn run_layer_with_events_internal<R: RngCore>(
    layer: &Layer,
    ctx: &LayerExecContext<'_>,
    cache: &FieldProgramCache,
    rng: &mut R,
    sink: &mut dyn EventSink,
    layer_index: usize,
) -> (RunResult, Option<(String, Arc<OverlayTexture>)>) {
    if layer.kinds.is_empty() {
        warn!("Layer '{}' has no kinds; skipping.", layer.id);
        if sink.wants(ScatterEventKind::Warning) {
            sink.send(ScatterEvent::Warning {
                context: format!("layer:{}", layer.id),
                message: "Layer has no kinds; skipping".into(),
            });
        }
        return (
            RunResult {
                placements: Vec::new(),
                positions_evaluated: 0,
                positions_rejected: 0,
            },
            None,
        );
    }

    let domain_extent = ctx.config.domain_extent;
    let domain_center = ctx.config.domain_center;

    let opts = CompileOptions::default();
    let mut kind_info: Vec<(Kind, Arc<FieldProgram>, Vec<String>, Option<String>)> = Vec::new();
    // Emit layer start
    if sink.wants(ScatterEventKind::LayerStarted) {
        sink.send(ScatterEvent::LayerStarted {
            index: layer_index,
            id: layer.id.clone(),
            kinds: layer.kinds.iter().map(|k| k.id.clone()).collect(),
            overlay_mask_size_px: layer.overlay_mask_size_px,
            overlay_brush_radius_px: layer.overlay_brush_radius_px,
        });
    }
    for k in &layer.kinds {
        match cache.get_or_compile(k, &opts) {
            Ok(program) => {
                let gates: Vec<String> = program
                    .nodes
                    .iter()
                    .filter_map(|(id, meta)| {
                        if meta.is_gate() {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                let prob_ids: Vec<String> = program
                    .nodes
                    .iter()
                    .filter(|(_, m)| m.is_probability())
                    .map(|(id, _)| id.clone())
                    .collect();
                if prob_ids.len() > 1 {
                    warn!(
                        "Kind '{}' has multiple Probability fields; using the first: {:?}.",
                        k.id, prob_ids
                    );
                    if sink.wants(ScatterEventKind::Warning) {
                        sink.send(ScatterEvent::Warning {
                            context: format!("layer:{} kind:{}", layer.id, k.id),
                            message: format!(
                                "Multiple Probability fields found; using first: {prob_ids:?}"
                            ),
                        });
                    }
                }
                let prob: Option<String> = prob_ids.into_iter().next();
                kind_info.push((k.clone(), program.clone(), gates, prob));
            }
            Err(e) => {
                warn!(
                    "Failed to compile kind '{}' in layer '{}': {}.",
                    k.id, layer.id, e
                );
                if sink.wants(ScatterEventKind::Warning) {
                    sink.send(ScatterEvent::Warning {
                        context: format!("layer:{} kind:{}", layer.id, k.id),
                        message: format!("Failed to compile kind: {e}"),
                    });
                }
            }
        }
    }
    if kind_info.is_empty() {
        return (
            RunResult {
                placements: Vec::new(),
                positions_evaluated: 0,
                positions_rejected: 0,
            },
            None,
        );
    }

    let positions_mint = layer.sampling.generate(domain_extent.into(), rng);
    let positions: Vec<Vec2> = positions_mint
        .into_iter()
        .map(Vec2::from)
        .map(|p| p + domain_center)
        .collect();

    let mut layer_textures =
        TextureRegistry::with_capacity(ctx.base_textures.len() + ctx.overlays.len());
    layer_textures.extend_from(ctx.base_textures);
    for (name, ov) in ctx.overlays.iter() {
        layer_textures.register_arc(name.clone(), ov.clone());
    }

    let mut runtime_cache: std::collections::HashMap<(KindId, ChunkId), FieldRuntime> =
        std::collections::HashMap::new();

    let mut placed: Vec<Placement> = Vec::new();
    for position in positions.iter().copied() {
        let (chunk, grid) = chunk::chunk_id_and_grid_for_position_in_domain(
            position,
            domain_extent,
            domain_center,
            ctx.config.chunk_extent,
            ctx.config.raster_cell_size,
            ctx.config.grid_halo,
        );

        let mut results: Vec<KindEvaluation> = Vec::with_capacity(kind_info.len());
        for (kind, program, gate_fields, probability_field) in &kind_info {
            let key = (kind.id.clone(), chunk);
            if !runtime_cache.contains_key(&key) {
                runtime_cache.insert(
                    key.clone(),
                    FieldRuntime::new(program.clone(), &layer_textures),
                );
            }
            let rt = runtime_cache
                .get_mut(&key)
                .expect("runtime exists after insertion");

            let mut allowed = true;
            for field_id in gate_fields {
                let value = rt.sample(field_id, position, chunk, &grid);
                if value <= 0.0 {
                    allowed = false;
                    break;
                }
            }

            let weight = if allowed {
                if let Some(prob_id) = probability_field {
                    rt.sample(prob_id, position, chunk, &grid).clamp(0.0, 1.0)
                } else {
                    DEFAULT_PROBABILITY_WHEN_MISSING
                }
            } else {
                0.0
            };

            results.push(KindEvaluation {
                kind: kind.clone(),
                allowed,
                weight,
            });
        }

        let max_weight = results
            .iter()
            .filter(|r| r.allowed)
            .map(|r| r.weight)
            .fold(0.0f32, f32::max);

        if sink.wants(ScatterEventKind::PositionEvaluated) {
            sink.send(ScatterEvent::PositionEvaluated {
                layer_index,
                layer_id: layer.id.clone(),
                position,
                evaluations: results
                    .iter()
                    .map(|r| {
                        crate::scatter::events::KindEvaluationLite::new(
                            r.kind.id.clone(),
                            r.allowed,
                            r.weight,
                        )
                    })
                    .collect(),
                max_weight,
            });
        }

        let rand01 = crate::sampling::rand01(rng);
        if max_weight > 0.0 && rand01 < max_weight {
            let selected = match layer.selection_strategy {
                SelectionStrategy::WeightedRandom => pick_weighted_random(&results, rng),
                SelectionStrategy::HighestProbability => pick_highest_probability(&results),
            };
            if let Some(selected_kind) = selected {
                let placement = Placement {
                    kind_id: selected_kind.id.clone(),
                    position,
                };
                if sink.wants(ScatterEventKind::PlacementMade) {
                    sink.send(ScatterEvent::PlacementMade {
                        layer_index,
                        layer_id: layer.id.clone(),
                        placement: placement.clone(),
                    });
                }
                placed.push(placement);
            }
        }
    }

    let eval_count = positions.len();
    let placed_count = placed.len();
    let rejected = eval_count.saturating_sub(placed_count);

    let overlay_opt = if let (Some((mask_w, mask_h)), Some(brush_radius)) =
        (layer.overlay_mask_size_px, layer.overlay_brush_radius_px)
    {
        if mask_w == 0 || mask_h == 0 {
            warn!(
                "Layer '{}' overlay size is zero; skipping overlay.",
                layer.id
            );
            if sink.wants(ScatterEventKind::Warning) {
                sink.send(ScatterEvent::Warning {
                    context: format!("layer:{}", layer.id),
                    message: "Overlay size is zero; skipping overlay".into(),
                });
            }
            None
        } else if brush_radius < 0 {
            warn!(
                "Layer '{}' overlay brush radius < 0; skipping overlay.",
                layer.id
            );
            if sink.wants(ScatterEventKind::Warning) {
                sink.send(ScatterEvent::Warning {
                    context: format!("layer:{}", layer.id),
                    message: "Overlay brush radius < 0; skipping overlay".into(),
                });
            }
            None
        } else {
            let mask = build_overlay_mask_from_positions_in_domain(
                domain_extent,
                domain_center,
                &placed.iter().map(|p| p.position).collect::<Vec<_>>(),
                mask_w,
                mask_h,
                brush_radius,
            );
            let mask_name = format!("mask_{}", layer.id);
            let summary = OverlaySummary {
                name: mask_name.clone(),
                size_px: (mask_w, mask_h),
            };
            if sink.wants(ScatterEventKind::OverlayGenerated) {
                sink.send(ScatterEvent::OverlayGenerated {
                    layer_index,
                    layer_id: layer.id.clone(),
                    summary: summary.clone(),
                });
            }
            Some((mask_name, Arc::new(mask)))
        }
    } else {
        None
    };

    (
        RunResult {
            placements: placed,
            positions_evaluated: eval_count,
            positions_rejected: rejected,
        },
        overlay_opt,
    )
}

pub fn run_plan<R: RngCore>(
    plan: &Plan,
    config: &RunConfig,
    base_textures: &TextureRegistry,
    cache: &FieldProgramCache,
    rng: &mut R,
    sink: Option<&mut dyn EventSink>,
) -> RunResult {
    if let Some(s) = sink {
        run_plan_with_events(plan, config, base_textures, cache, rng, s)
    } else {
        run_plan_with_events(plan, config, base_textures, cache, rng, &mut ())
    }
}

pub fn run_plan_with_events<R: RngCore>(
    plan: &Plan,
    config: &RunConfig,
    base_textures: &TextureRegistry,
    cache: &FieldProgramCache,
    rng: &mut R,
    sink: &mut dyn EventSink,
) -> RunResult {
    if sink.wants(ScatterEventKind::RunStarted) {
        sink.send(ScatterEvent::RunStarted {
            config: config.clone(),
            layer_count: plan.layers.len(),
        });
    }

    if plan.layers.is_empty() {
        warn!("Placement plan has no layers.");
        if sink.wants(ScatterEventKind::Warning) {
            sink.send(ScatterEvent::Warning {
                context: "plan".into(),
                message: "Placement plan has no layers".into(),
            });
        }
    }

    let mut overlays: HashMap<String, Arc<OverlayTexture>> = HashMap::new();

    let mut all_placed: Vec<Placement> = Vec::new();
    let mut total_eval = 0;
    let mut total_reject = 0;

    for (layer_idx, layer) in plan.layers.iter().enumerate() {
        info!(
            "Layer {}: '{}' | kinds: {}.",
            layer_idx,
            layer.id,
            layer.kinds.len(),
        );

        let ctx = LayerExecContext {
            config,
            base_textures,
            overlays: &overlays,
        };
        let (layer_result, overlay_opt) =
            run_layer_with_events_internal(layer, &ctx, cache, rng, sink, layer_idx);

        total_eval += layer_result.positions_evaluated;
        total_reject += layer_result.positions_rejected;
        all_placed.extend(layer_result.placements.iter().cloned());

        let overlay_summary = overlay_opt.as_ref().map(|(name, texture)| OverlaySummary {
            name: name.clone(),
            size_px: (texture.width, texture.height),
        });

        if sink.wants(ScatterEventKind::LayerFinished) {
            sink.send(ScatterEvent::LayerFinished {
                index: layer_idx,
                id: layer.id.clone(),
                result: layer_result.clone(),
                overlay: overlay_summary.clone(),
            });
        }

        if let Some((name, ov)) = overlay_opt {
            overlays.insert(name, ov);
        }
    }

    let result = RunResult {
        placements: all_placed,
        positions_evaluated: total_eval,
        positions_rejected: total_reject,
    };

    if sink.wants(ScatterEventKind::RunFinished) {
        sink.send(ScatterEvent::RunFinished {
            result: result.clone(),
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;
    use crate::fieldgraph::spec::{FieldGraphSpec, FieldSemantics};
    use crate::fieldgraph::NodeSpec;
    use crate::sampling::JitterGridSampling;
    use crate::scatter::events::{ScatterEvent, VecSink};

    fn make_kind(id: &str) -> Kind {
        let mut spec = FieldGraphSpec::default();
        spec.add_with_semantics(
            "probability",
            NodeSpec::constant(1.0),
            FieldSemantics::Probability,
        );
        Kind::new(id, spec)
    }

    fn base_config() -> RunConfig {
        RunConfig::new(Vec2::new(10.0, 10.0))
            .with_chunk_extent(10.0)
            .with_raster_cell_size(5.0)
            .with_grid_halo(0)
    }

    #[test]
    fn layer_events_use_supplied_index() {
        let cache = FieldProgramCache::new();
        let textures = TextureRegistry::new();
        let mut rng = StdRng::seed_from_u64(42);

        let layer_a = Layer::new_with(
            "layer_a",
            vec![make_kind("kind_a")],
            JitterGridSampling::new(0.0, 5.0),
        );
        let layer_b = Layer::new_with(
            "layer_b",
            vec![make_kind("kind_b")],
            JitterGridSampling::new(0.0, 5.0),
        );
        let plan = Plan::new().with_layers(vec![layer_a, layer_b]);

        let mut sink = VecSink::new();
        run_plan_with_events(
            &plan,
            &base_config(),
            &textures,
            &cache,
            &mut rng,
            &mut sink,
        );

        let events = sink.into_inner();
        let started_indices: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                ScatterEvent::LayerStarted { index, .. } => Some(*index),
                _ => None,
            })
            .collect();
        assert_eq!(started_indices, vec![0, 1]);

        let placement_indices: HashSet<_> = events
            .iter()
            .filter_map(|event| match event {
                ScatterEvent::PlacementMade { layer_index, .. } => Some(*layer_index),
                _ => None,
            })
            .collect();
        assert!(placement_indices.contains(&0));
        assert!(placement_indices.contains(&1));
    }

    #[test]
    fn layer_finished_reports_overlay_dimensions() {
        let cache = FieldProgramCache::new();
        let textures = TextureRegistry::new();
        let mut rng = StdRng::seed_from_u64(7);

        let layer = Layer::new_with(
            "overlay_layer",
            vec![make_kind("kind_overlay")],
            JitterGridSampling::new(0.0, 5.0),
        )
        .with_overlay((8, 8), 2);

        let plan = Plan::new().with_layer(layer);

        let mut sink = VecSink::new();
        run_plan_with_events(
            &plan,
            &base_config(),
            &textures,
            &cache,
            &mut rng,
            &mut sink,
        );

        let overlay_size = sink
            .into_inner()
            .into_iter()
            .find_map(|event| match event {
                ScatterEvent::LayerFinished {
                    id,
                    overlay: Some(summary),
                    ..
                } if id == "overlay_layer" => Some(summary.size_px),
                _ => None,
            })
            .expect("expected overlay summary");

        assert_eq!(overlay_size, (8, 8));
    }
}
