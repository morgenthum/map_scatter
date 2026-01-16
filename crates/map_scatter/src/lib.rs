#![forbid(unsafe_code)]
//! map_scatter: Rule-based object scattering with field-graph evaluation and sampling.
//!
//! Modules:
//! - fieldgraph: author, compile, and evaluate scalar field DAGs (incl. textures and EDT normalization)
//! - sampling: candidate generation (jitter grid, Poisson disk)
//! - scatter: plans, layers, runner, selection, overlays, events
//!
//! For examples and docs, see README and docs.rs.
pub mod error;
pub mod fieldgraph;
pub mod sampling;
pub mod scatter;

/// Convenient re-exports for common types. Import with `use map_scatter::prelude::*;`.
pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::fieldgraph::cache::FieldProgramCache;
    pub use crate::fieldgraph::compiler::{CompileOptions, FieldGraphCompiler};
    pub use crate::fieldgraph::spec::{FieldGraphSpec, FieldSemantics};
    pub use crate::fieldgraph::{NodeSpec, Texture, TextureChannel, TextureRegistry};
    pub use crate::sampling::{
        BestCandidateSampling, ClusteredSampling, FibonacciLatticeSampling, HaltonSampling,
        HexJitterGridSampling, JitterGridSampling, PoissonDiskSampling, PositionSampling,
        StratifiedMultiJitterSampling, UniformRandomSampling,
    };
    pub use crate::scatter::chunk::seed_for_chunk;
    pub use crate::scatter::events::{
        AsEventSink, EventSink, FnSink, KindEvaluationLite, MultiSink, OverlaySummary,
        ScatterEvent, ScatterEventKind, VecSink,
    };
    pub use crate::scatter::overlay::OverlayTexture;
    pub use crate::scatter::plan::{Layer, Plan, SelectionStrategy};
    pub use crate::scatter::runner::{
        run_layer, run_plan, Placement, RunConfig, RunResult, ScatterRunner,
    };
    pub use crate::scatter::selection::{pick_highest_probability, pick_weighted_random};
    pub use crate::scatter::{Kind, KindId};
}
