//! Cache for compiled field graph programs.
//!
//! This module provides a cache that maps [`KindId`] to compiled [`FieldProgram`]s,
//! recompiling entries when the associated [`FieldGraphSpec`] fingerprint changes.
//!
//! Typical usage:
//! - Look up a program with [`FieldProgramCache::get_or_compile`] by passing a [`Kind`]
//!   and [`CompileOptions`].
//! - Reuse cached programs across scatter runs to avoid recompilation.
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::error::{Error, Result};
use crate::fieldgraph::compiler::{CompileOptions, FieldGraphCompiler};
use crate::fieldgraph::FieldProgram;
use crate::prelude::{FieldGraphSpec, FieldSemantics, NodeSpec, TextureChannel};
use crate::scatter::{Kind, KindId};

struct ProgramEntry {
    program: FieldProgram,
    fingerprint: u64,
}

/// Cache for compiled field programs, keyed by [`KindId`] and invalidated by specification fingerprint.
pub struct FieldProgramCache {
    entries: HashMap<KindId, ProgramEntry>,
}

impl FieldProgramCache {
    /// Creates a new, empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Gets a reference to the compiled program for the given [`KindId`], if it exists in the cache.
    pub fn get_for_kind(&self, kind_id: KindId) -> Option<&FieldProgram> {
        self.entries.get(&kind_id).map(|e| &e.program)
    }

    /// Inserts a compiled program into the cache with the given [`KindId`] and specification fingerprint.
    pub fn insert(&mut self, kind_id: KindId, fingerprint: u64, program: FieldProgram) {
        self.entries.insert(
            kind_id,
            ProgramEntry {
                fingerprint,
                program,
            },
        );
    }

    /// Removes the compiled program for the given [`KindId`] from the cache, returning it if it existed.
    pub fn remove(&mut self, kind_id: KindId) -> Option<FieldProgram> {
        self.entries.remove(&kind_id).map(|e| e.program)
    }

    /// Clears all entries from the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Gets the compiled program for the given [`Kind`], compiling and caching it if necessary.
    pub fn get_or_compile<'a>(
        &'a mut self,
        kind: &Kind,
        opts: &CompileOptions,
    ) -> Result<&'a FieldProgram> {
        let key = &kind.id;
        let fp = fingerprint(&kind.spec, opts);

        let needs_compile = match self.entries.get(key) {
            Some(entry) => entry.fingerprint != fp,
            None => true,
        };

        if needs_compile {
            let program = FieldGraphCompiler::compile(&kind.spec, opts)?;
            self.insert(key.clone(), fp, program);
        }

        match self.entries.get(key) {
            Some(entry) => Ok(&entry.program),
            None => Err(Error::Other("Entry missing after insert".to_string())),
        }
    }
}

impl Default for FieldProgramCache {
    fn default() -> Self {
        Self::new()
    }
}

fn fingerprint(spec: &FieldGraphSpec, opts: &CompileOptions) -> u64 {
    let mut hasher = DefaultHasher::new();

    let mut ids: Vec<&String> = spec.nodes.keys().collect();
    ids.sort();

    for id in ids {
        id.hash(&mut hasher);
        let node = &spec.nodes[id];

        let kind_tag: u8 = match node {
            NodeSpec::Constant { .. } => 1,
            NodeSpec::Texture { .. } => 2,
            NodeSpec::Add { .. } => 3,
            NodeSpec::Mul { .. } => 4,
            NodeSpec::Min { .. } => 5,
            NodeSpec::Max { .. } => 6,
            NodeSpec::Invert { .. } => 7,
            NodeSpec::Clamp { .. } => 8,
            NodeSpec::SmoothStep { .. } => 9,
            NodeSpec::Pow { .. } => 10,
            NodeSpec::EdtNormalize { .. } => 11,
            NodeSpec::Sub { .. } => 12,
            NodeSpec::Scale { .. } => 13,
        };
        kind_tag.hash(&mut hasher);

        let semantics_tag: u8 = match spec.semantics.get(id) {
            Some(s) => match s {
                FieldSemantics::Gate => 0,
                FieldSemantics::Probability => 1,
            },
            None => 255,
        };
        semantics_tag.hash(&mut hasher);

        for input in node.inputs() {
            input.hash(&mut hasher);
        }

        match node {
            NodeSpec::Constant { params } => {
                params.value.to_bits().hash(&mut hasher);
            }
            NodeSpec::Texture { params } => {
                params.texture_id.hash(&mut hasher);
                let channel_tag: u8 = match params.channel {
                    TextureChannel::R => 0,
                    TextureChannel::G => 1,
                    TextureChannel::B => 2,
                    TextureChannel::A => 3,
                };
                channel_tag.hash(&mut hasher);
            }
            NodeSpec::Scale { params, .. } => {
                params.factor.to_bits().hash(&mut hasher);
            }
            NodeSpec::Clamp { params, .. } => {
                params.min.to_bits().hash(&mut hasher);
                params.max.to_bits().hash(&mut hasher);
            }
            NodeSpec::SmoothStep { params, .. } => {
                params.edge0.to_bits().hash(&mut hasher);
                params.edge1.to_bits().hash(&mut hasher);
            }
            NodeSpec::Pow { params, .. } => {
                params.exp.to_bits().hash(&mut hasher);
            }
            NodeSpec::EdtNormalize { params, .. } => {
                params.threshold.to_bits().hash(&mut hasher);
                params.d_max.to_bits().hash(&mut hasher);
            }
            _ => {}
        }
    }

    if !opts.force_bake.is_empty() {
        let mut bake_ids: Vec<&str> = opts.force_bake.iter().map(|s| s.as_str()).collect();
        bake_ids.sort_unstable();
        for id in bake_ids {
            id.hash(&mut hasher);
        }
    }

    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind_with_constant(id: &str, value: f32) -> Kind {
        let mut spec = FieldGraphSpec::default();
        spec.add_with_semantics(
            "prob",
            NodeSpec::constant(value),
            FieldSemantics::Probability,
        );
        Kind::new(id, spec)
    }

    fn constant_from_program(program: &FieldProgram) -> f32 {
        if let Some(meta) = program.nodes.get("prob") {
            if let NodeSpec::Constant { params } = &meta.spec {
                return params.value;
            }
        }
        panic!("expected constant node");
    }

    #[test]
    fn caches_and_returns_compiled_programs() {
        let mut cache = FieldProgramCache::new();
        let kind = kind_with_constant("tree", 0.5);
        let program = cache
            .get_or_compile(&kind, &CompileOptions::default())
            .expect("compile succeeds");

        assert_eq!(constant_from_program(program), 0.5);
        assert!(cache.get_for_kind(kind.id.clone()).is_some());

        // Removing should drop the entry.
        let removed = cache.remove(kind.id.clone());
        assert!(removed.is_some());
        assert!(cache.get_for_kind(kind.id.clone()).is_none());

        // Reinserting via insert works as well.
        let opts = CompileOptions::default();
        let program = FieldGraphCompiler::compile(&kind.spec, &opts).unwrap();
        cache.insert(kind.id.clone(), fingerprint(&kind.spec, &opts), program);
        assert!(cache.get_for_kind(kind.id.clone()).is_some());
    }

    #[test]
    fn recompiles_when_spec_fingerprint_changes() {
        let mut cache = FieldProgramCache::new();

        let kind_v1 = kind_with_constant("rock", 0.3);
        let program_v1 = cache
            .get_or_compile(&kind_v1, &CompileOptions::default())
            .expect("first compile succeeds");
        assert_eq!(constant_from_program(program_v1), 0.3);

        let kind_v2 = kind_with_constant("rock", 0.9);
        let program_v2 = cache
            .get_or_compile(&kind_v2, &CompileOptions::default())
            .expect("second compile succeeds");
        assert_eq!(constant_from_program(program_v2), 0.9);
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut cache = FieldProgramCache::new();

        let kind = kind_with_constant("bush", 0.2);
        cache
            .get_or_compile(&kind, &CompileOptions::default())
            .expect("compile succeeds");
        assert!(cache.get_for_kind(kind.id.clone()).is_some());

        cache.clear();
        assert!(cache.get_for_kind(kind.id.clone()).is_none());
    }

    #[test]
    fn recompiles_when_compile_options_change() {
        let mut cache = FieldProgramCache::new();
        let kind = kind_with_constant("grass", 0.5);

        let opts_a = CompileOptions::default();
        let program_a = cache
            .get_or_compile(&kind, &opts_a)
            .expect("initial compile succeeds");
        assert!(!program_a.nodes.get("prob").expect("node exists").force_bake);

        let mut opts_b = CompileOptions::default();
        opts_b.force_bake.insert("prob".into());
        let program_b = cache
            .get_or_compile(&kind, &opts_b)
            .expect("force bake compile succeeds");
        assert!(program_b.nodes.get("prob").expect("node exists").force_bake);
    }
}
