//! Scattering pipeline for evaluating spatial fields and placing kinds across a 2D domain.
use crate::fieldgraph::spec::FieldGraphSpec;

pub mod chunk;
pub mod evaluator;
pub mod events;
pub mod overlay;
pub mod plan;
pub mod runner;
pub mod selection;

pub const DEFAULT_PROBABILITY_WHEN_MISSING: f32 = 0.1;

pub type KindId = String;

/// Represents a type of object to be scattered, defined by a unique identifier and a
/// specification of the field graph that determines its placement.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct Kind {
    pub id: KindId,
    pub spec: FieldGraphSpec,
}

impl Kind {
    pub fn new(id: impl Into<KindId>, spec: FieldGraphSpec) -> Self {
        Self {
            id: id.into(),
            spec,
        }
    }
}
