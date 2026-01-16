//! Program representation for compiled field graphs.
//!
//! This module defines the data structures produced by compiling a
//! [`crate::fieldgraph::spec::FieldGraphSpec`] of [`crate::fieldgraph::NodeSpec`]s into an
//! executable program.
use std::collections::HashMap;

pub use crate::fieldgraph::spec::FieldSemantics;
pub use crate::fieldgraph::{FieldId, NodeSpec};

/// Metadata about a node in the field program.
#[derive(Clone, Debug)]
pub struct NodeMeta {
    /// Field id for this node.
    pub id: FieldId,
    /// Node specification for this field.
    pub spec: NodeSpec,
    /// Whether this node should be baked into a raster.
    pub force_bake: bool,
    /// Optional semantic tag for this field.
    pub semantics: Option<FieldSemantics>,
}

impl NodeMeta {
    /// Check if the node has gate semantics.
    #[inline]
    pub fn is_gate(&self) -> bool {
        matches!(self.semantics, Some(FieldSemantics::Gate))
    }

    /// Check if the node has probability semantics.
    #[inline]
    pub fn is_probability(&self) -> bool {
        matches!(self.semantics, Some(FieldSemantics::Probability))
    }
}

/// A field program, consisting of nodes and their topological order.
#[derive(Clone, Debug)]
pub struct FieldProgram {
    /// Node metadata keyed by field id.
    pub nodes: HashMap<FieldId, NodeMeta>,
    /// Topological order of node evaluation.
    pub topo: Vec<FieldId>,
}
