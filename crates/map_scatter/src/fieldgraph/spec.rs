//! Specification types for authoring field graphs.
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::fieldgraph::{FieldId, NodeSpec};

/// A specification of a field graph, including nodes and their semantics.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct FieldGraphSpec {
    pub nodes: HashMap<FieldId, NodeSpec>,
    pub semantics: HashMap<FieldId, FieldSemantics>,
}

impl FieldGraphSpec {
    /// Add a node to the field graph specification.
    pub fn add(&mut self, id: &str, spec: NodeSpec) -> &mut Self {
        self.nodes.insert(id.to_string(), spec);
        self
    }

    /// Set the semantics for a node in the field graph specification.
    pub fn set_semantics(&mut self, id: &str, semantics: FieldSemantics) -> &mut Self {
        self.semantics.insert(id.to_string(), semantics);
        self
    }

    /// Add a node with semantics to the field graph specification.
    pub fn add_with_semantics(
        &mut self,
        id: &str,
        spec: NodeSpec,
        semantics: FieldSemantics,
    ) -> &mut Self {
        self.add(id, spec);
        self.set_semantics(id, semantics);
        self
    }
}

/// The semantics of a field node, indicating its role in the field graph.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldSemantics {
    Gate,
    Probability,
}
