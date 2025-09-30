//! Compiler for field graph specifications into executable programs.
//!
//! This module turns a [`FieldGraphSpec`] into a runnable [`FieldProgram`].
//! It performs input validation, marks nodes requested for baking
//! via [`CompileOptions`], and computes a topological order for evaluation.
//!
//! Typical usage:
//! - [`FieldGraphCompiler`] with [`FieldGraphCompiler::compile`]
use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result};
use crate::fieldgraph::{FieldId, FieldProgram, NodeMeta, NodeSpec};
use crate::prelude::FieldGraphSpec;

/// Options for compiling a field graph.
#[derive(Clone, Debug, Default)]
pub struct CompileOptions {
    /// Set of field IDs that should be forced to be baked.
    pub force_bake: HashSet<FieldId>,
}

/// Compiler for field graph specifications into executable programs.
pub struct FieldGraphCompiler;

impl FieldGraphCompiler {
    /// Compiles a field graph specification into a [`FieldProgram`], applying the given options.
    pub fn compile(spec: &FieldGraphSpec, opts: &CompileOptions) -> Result<FieldProgram> {
        let mut nodes: HashMap<FieldId, NodeMeta> = HashMap::new();

        for (id, node_spec) in &spec.nodes {
            for input in node_spec.inputs() {
                if !spec.nodes.contains_key(input) {
                    return Err(Error::Compile(format!(
                        "Node '{}' references unknown input '{}'",
                        id, input
                    )));
                }
            }

            validate_node_inputs(id, node_spec)?;

            let force_bake = opts.force_bake.contains(id);

            nodes.insert(
                id.clone(),
                NodeMeta {
                    id: id.clone(),
                    spec: node_spec.clone(),
                    force_bake,
                    semantics: spec.semantics.get(id).cloned(),
                },
            );
        }

        let topo = topo_sort(&nodes)?;
        Ok(FieldProgram { nodes, topo })
    }
}

fn validate_node_inputs(id: &str, node_spec: &NodeSpec) -> Result<()> {
    let inputs = node_spec.inputs();

    let ensure_at_least_one = |variant: &str| {
        if inputs.is_empty() {
            Err(Error::Compile(format!(
                "Node '{}' ({}) requires at least one input",
                id, variant
            )))
        } else {
            Ok(())
        }
    };

    let ensure_exactly_one = |variant: &str| {
        if inputs.len() != 1 {
            Err(Error::Compile(format!(
                "Node '{}' ({}) requires exactly one input but found {}",
                id,
                variant,
                inputs.len()
            )))
        } else {
            Ok(())
        }
    };

    match node_spec {
        NodeSpec::Constant { .. } | NodeSpec::Texture { .. } => Ok(()),
        NodeSpec::Add { .. } => ensure_at_least_one("Add"),
        NodeSpec::Sub { .. } => ensure_at_least_one("Sub"),
        NodeSpec::Mul { .. } => ensure_at_least_one("Mul"),
        NodeSpec::Min { .. } => ensure_at_least_one("Min"),
        NodeSpec::Max { .. } => ensure_at_least_one("Max"),
        NodeSpec::Invert { .. } => ensure_exactly_one("Invert"),
        NodeSpec::Scale { .. } => ensure_exactly_one("Scale"),
        NodeSpec::Clamp { .. } => ensure_exactly_one("Clamp"),
        NodeSpec::SmoothStep { .. } => ensure_exactly_one("SmoothStep"),
        NodeSpec::Pow { .. } => ensure_exactly_one("Pow"),
        NodeSpec::EdtNormalize { .. } => ensure_exactly_one("EdtNormalize"),
    }
}

fn topo_sort(nodes: &HashMap<FieldId, NodeMeta>) -> Result<Vec<FieldId>> {
    let mut indeg: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, HashMap<&str, usize>> = HashMap::new();

    for (id, meta) in nodes {
        let id_str = id.as_str();
        let inputs = meta.spec.inputs();
        indeg.insert(id_str, inputs.len());

        for input in inputs {
            dependents
                .entry(input.as_str())
                .or_default()
                .entry(id_str)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
    }

    let mut q: Vec<&str> = indeg
        .iter()
        .filter_map(|(k, &v)| if v == 0 { Some(*k) } else { None })
        .collect();
    let mut out: Vec<FieldId> = Vec::new();

    while let Some(n) = q.pop() {
        out.push(n.to_string());

        if let Some(children) = dependents.get(n) {
            for (child, count) in children {
                if let Some(e) = indeg.get_mut(child) {
                    *e = e.saturating_sub(*count);
                    if *e == 0 {
                        q.push(child);
                    }
                }
            }
        }
    }

    if out.len() != nodes.len() {
        return Err(Error::Compile("Cycle detected or missing nodes".into()));
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fieldgraph::node::{PowParams, ScaleParams};
    use crate::prelude::{FieldSemantics, NodeSpec};

    #[test]
    fn compile_orders_nodes_topologically() {
        let mut spec = FieldGraphSpec::default();
        spec.add("a", NodeSpec::constant(1.0));
        spec.add("b", NodeSpec::add(vec!["a".into()]));
        spec.add_with_semantics(
            "prob",
            NodeSpec::mul(vec!["a".into(), "b".into()]),
            FieldSemantics::Probability,
        );

        let opts = CompileOptions::default();
        let program = FieldGraphCompiler::compile(&spec, &opts).expect("compile succeeds");

        assert_eq!(program.nodes.len(), 3);

        // Topological order should place dependencies first
        let pos_a = program.topo.iter().position(|id| id == "a").unwrap();
        let pos_b = program.topo.iter().position(|id| id == "b").unwrap();
        let pos_prob = program.topo.iter().position(|id| id == "prob").unwrap();

        assert!(pos_a < pos_b && pos_b < pos_prob);
    }

    #[test]
    fn compile_detects_unknown_inputs() {
        let mut spec = FieldGraphSpec::default();
        spec.add("bad", NodeSpec::add(vec!["missing".into()]));

        let err = FieldGraphCompiler::compile(&spec, &CompileOptions::default())
            .expect_err("expected compile failure");
        matches!(err, Error::Compile(_))
            .then_some(())
            .expect("compile error");
    }

    #[test]
    fn compile_rejects_nodes_with_missing_inputs() {
        let mut spec = FieldGraphSpec::default();
        spec.add("a", NodeSpec::constant(1.0));
        spec.add("bad_min", NodeSpec::min(Vec::new()));

        let err = FieldGraphCompiler::compile(&spec, &CompileOptions::default())
            .expect_err("missing inputs should fail");
        matches!(err, Error::Compile(_))
            .then_some(())
            .expect("compile error");
    }

    #[test]
    fn compile_rejects_nodes_with_extra_inputs() {
        let mut spec = FieldGraphSpec::default();
        spec.add("a", NodeSpec::constant(1.0));
        spec.add(
            "bad_scale",
            NodeSpec::Scale {
                inputs: vec!["a".into(), "a".into()],
                params: ScaleParams { factor: 2.0 },
            },
        );
        spec.add(
            "bad_pow",
            NodeSpec::Pow {
                inputs: vec![],
                params: PowParams { exp: 2.0 },
            },
        );

        let err = FieldGraphCompiler::compile(&spec, &CompileOptions::default())
            .expect_err("extra or missing inputs should fail");
        matches!(err, Error::Compile(_))
            .then_some(())
            .expect("compile error");
    }

    #[test]
    fn compile_detects_cycles() {
        let mut spec = FieldGraphSpec::default();
        spec.add("a", NodeSpec::add(vec!["b".into()]));
        spec.add("b", NodeSpec::add(vec!["a".into()]));

        let err = FieldGraphCompiler::compile(&spec, &CompileOptions::default())
            .expect_err("cycle should fail");
        matches!(err, Error::Compile(_))
            .then_some(())
            .expect("compile error");
    }

    #[test]
    fn compile_marks_force_bake_nodes() {
        let mut spec = FieldGraphSpec::default();
        spec.add("base", NodeSpec::constant(0.0));
        spec.add("baked", NodeSpec::scale("base".into(), 2.0));

        let mut opts = CompileOptions::default();
        opts.force_bake.insert("baked".into());

        let program = FieldGraphCompiler::compile(&spec, &opts).expect("compile succeeds");
        assert!(program.nodes.get("baked").expect("node exists").force_bake);
    }

    #[test]
    fn compile_handles_duplicate_inputs() {
        let mut spec = FieldGraphSpec::default();
        spec.add("a", NodeSpec::constant(1.0));
        spec.add("square", NodeSpec::mul(vec!["a".into(), "a".into()]));

        let program = FieldGraphCompiler::compile(&spec, &CompileOptions::default())
            .expect("compile succeeds");

        assert_eq!(program.topo.len(), 2);
        assert!(program.topo.iter().any(|f| f == "square"));
    }
}
