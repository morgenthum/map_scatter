//! Evaluator for kinds at positions based on field graphs.
//!
//! This module prepares and evaluates per-kind gate and probability fields for
//! candidate positions. It compiles each [`Kind`]'s [`crate::fieldgraph::spec::FieldGraphSpec`]
//! into a [`FieldProgram`], identifies fields by [`crate::fieldgraph::spec::FieldSemantics`],
//! and samples values through a [`FieldRuntime`].
use std::collections::HashMap;
use std::sync::Arc;

use glam::Vec2;

use crate::error::{Error, Result};
use crate::fieldgraph::cache::FieldProgramCache;
use crate::fieldgraph::compiler::CompileOptions;
use crate::fieldgraph::program::FieldProgram;
use crate::fieldgraph::runtime::FieldRuntime;
use crate::fieldgraph::{ChunkGrid, ChunkId, TextureRegistry};
use crate::scatter::{Kind, DEFAULT_PROBABILITY_WHEN_MISSING};

/// Result of evaluating a [`Kind`] at a position.
#[derive(Debug, Clone)]
pub struct KindEvaluation {
    /// Kind being evaluated.
    pub kind: Kind,
    /// Whether all gate fields passed for this position.
    pub allowed: bool,
    /// Final probability weight in [0, 1] used for selection.
    pub weight: f32,
}

struct KindInfo {
    program: Arc<FieldProgram>,
    gate_fields: Vec<String>,
    probability_field: Option<String>,
}

/// Evaluator for kinds at positions based on their field graphs.
pub struct Evaluator {
    kind_info: HashMap<String, KindInfo>,
}

impl Evaluator {
    /// Creates a new evaluator by compiling the field graphs of the given kinds.
    pub fn new(kinds: &[Kind], cache: &FieldProgramCache) -> Result<Self> {
        let mut kind_info = HashMap::new();
        let opts = CompileOptions::default();

        for kind in kinds {
            let program = cache.get_or_compile(kind, &opts)?;

            let gate_fields: Vec<_> = program
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

            let prob_ids: Vec<_> = program
                .nodes
                .iter()
                .filter(|(_, m)| m.is_probability())
                .map(|(id, _)| id.clone())
                .collect();

            if prob_ids.len() > 1 {
                return Err(Error::Compile(format!(
                    "Kind '{}' has multiple Probability fields",
                    kind.id
                )));
            }
            let probability_field = prob_ids.into_iter().next();

            kind_info.insert(
                kind.id.clone(),
                KindInfo {
                    program: program.clone(),
                    gate_fields,
                    probability_field,
                },
            );
        }

        Ok(Self { kind_info })
    }

    /// Evaluates all kinds at a single position, returning a sorted list of evaluations.
    pub fn evaluate_position(
        &self,
        position: Vec2,
        chunk: ChunkId,
        grid: &ChunkGrid,
        kinds: &[Kind],
        textures: &TextureRegistry,
    ) -> Vec<KindEvaluation> {
        let results = self.evaluate_positions_batched(
            std::slice::from_ref(&position),
            chunk,
            grid,
            kinds,
            textures,
        );
        results.into_iter().next().unwrap_or_default()
    }

    /// Evaluates all kinds at multiple positions, returning a list of sorted evaluations per position.
    pub fn evaluate_positions_batched(
        &self,
        positions: &[Vec2],
        chunk: ChunkId,
        grid: &ChunkGrid,
        kinds: &[Kind],
        textures: &TextureRegistry,
    ) -> Vec<Vec<KindEvaluation>> {
        let mut runtimes: HashMap<String, FieldRuntime> = HashMap::new();

        for kind in kinds {
            if !runtimes.contains_key(&kind.id) {
                if let Some(info) = self.kind_info.get(&kind.id) {
                    runtimes.insert(
                        kind.id.clone(),
                        FieldRuntime::new(info.program.clone(), textures),
                    );
                }
            }
        }

        let mut all_results = Vec::with_capacity(positions.len());

        for &pos in positions {
            let mut results = Vec::with_capacity(kinds.len());

            for kind in kinds {
                if let Some(info) = self.kind_info.get(&kind.id) {
                    if let Some(rt) = runtimes.get_mut(&kind.id) {
                        let mut allowed = true;
                        for field_id in &info.gate_fields {
                            let value = rt.sample(field_id, pos, chunk, grid);
                            if value <= 0.0 {
                                allowed = false;
                                break;
                            }
                        }

                        let weight = if allowed {
                            if let Some(prob_id) = &info.probability_field {
                                rt.sample(prob_id, pos, chunk, grid).clamp(0.0, 1.0)
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
                }
            }

            results.sort_by(|a, b| b.weight.total_cmp(&a.weight));
            all_results.push(results);
        }

        all_results
    }

    /// Evaluates a single kind at a single position, returning the evaluation if the kind is known.
    pub fn evaluate_kind(
        &self,
        kind: &Kind,
        position: Vec2,
        chunk: ChunkId,
        grid: &ChunkGrid,
        textures: &TextureRegistry,
    ) -> Option<KindEvaluation> {
        let info = self.kind_info.get(&kind.id)?;
        let mut runtime = FieldRuntime::new(info.program.clone(), textures);

        let mut allowed = true;
        for field_id in &info.gate_fields {
            let value = runtime.sample(field_id, position, chunk, grid);
            if value <= 0.0 {
                allowed = false;
                break;
            }
        }

        let weight = if allowed {
            if let Some(prob_id) = &info.probability_field {
                runtime
                    .sample(prob_id, position, chunk, grid)
                    .clamp(0.0, 1.0)
            } else {
                DEFAULT_PROBABILITY_WHEN_MISSING
            }
        } else {
            0.0
        };

        Some(KindEvaluation {
            kind: kind.clone(),
            allowed,
            weight,
        })
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use super::*;
    use crate::fieldgraph::spec::{FieldGraphSpec, FieldSemantics};
    use crate::fieldgraph::NodeSpec;
    use crate::scatter::Kind;

    fn kind_allowed(id: &str, gate_value: f32, prob_value: Option<f32>) -> Kind {
        let mut spec = FieldGraphSpec::default();
        spec.add_with_semantics("gate", NodeSpec::constant(gate_value), FieldSemantics::Gate);
        if let Some(prob) = prob_value {
            spec.add_with_semantics(
                "prob",
                NodeSpec::constant(prob),
                FieldSemantics::Probability,
            );
        }
        Kind::new(id, spec)
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

    #[test]
    fn evaluator_applies_gate_and_probability() {
        let cache = FieldProgramCache::new();
        let kinds = vec![
            kind_allowed("allowed", 1.0, Some(0.6)),
            kind_allowed("blocked", 0.0, Some(0.9)),
        ];
        let evaluator = Evaluator::new(&kinds, &cache).expect("build evaluator");

        let results = evaluator.evaluate_position(
            Vec2::ZERO,
            ChunkId(0, 0),
            &grid(),
            &kinds,
            &TextureRegistry::new(),
        );

        assert_eq!(results.len(), 2);
        let allowed_eval = results.iter().find(|r| r.kind.id == "allowed").unwrap();
        assert!(allowed_eval.allowed);
        assert_eq!(allowed_eval.weight, 0.6);

        let blocked_eval = results.iter().find(|r| r.kind.id == "blocked").unwrap();
        assert!(!blocked_eval.allowed);
        assert_eq!(blocked_eval.weight, 0.0);
    }

    #[test]
    fn evaluator_defaults_probability_when_missing() {
        let cache = FieldProgramCache::new();
        let kinds = vec![kind_allowed("no_prob", 1.0, None)];
        let evaluator = Evaluator::new(&kinds, &cache).expect("build evaluator");

        let results = evaluator.evaluate_positions_batched(
            &[Vec2::ZERO, Vec2::new(1.0, 0.0)],
            ChunkId(0, 0),
            &grid(),
            &kinds,
            &TextureRegistry::new(),
        );

        assert_eq!(results.len(), 2);
        for eval in results.iter().flat_map(|v| v.iter()) {
            assert!(eval.allowed);
            assert_eq!(eval.weight, DEFAULT_PROBABILITY_WHEN_MISSING);
        }
    }

    #[test]
    fn evaluate_kind_returns_single_result() {
        let cache = FieldProgramCache::new();
        let kind = kind_allowed("single", 1.0, Some(0.3));
        let evaluator =
            Evaluator::new(std::slice::from_ref(&kind), &cache).expect("build evaluator");

        let result = evaluator
            .evaluate_kind(
                &kind,
                Vec2::ZERO,
                ChunkId(0, 0),
                &grid(),
                &TextureRegistry::new(),
            )
            .expect("kind evaluation");
        assert!(result.allowed);
        assert_eq!(result.weight, 0.3);
    }
}
