//! Field graph subsystem for defining and evaluating scalar fields used by the scatter pipeline.
//!
//! This module groups types for authoring a directed acyclic graph (DAG) of field nodes,
//! compiling it into an executable program, and evaluating it over chunked grids at runtime.
pub mod cache;
pub mod compiler;
pub mod edt;
pub mod grid;
pub mod node;
pub mod program;
pub mod raster;
pub mod runtime;
pub mod spec;
pub mod texture;

pub use grid::{ChunkGrid, ChunkId};
pub use node::{
    ClampParams, ConstantParams, EdtNormalizeParams, NodeSpec, PowParams, ScaleParams,
    SmoothStepParams, TextureParams,
};
pub use program::{FieldProgram, NodeMeta};
pub use raster::Raster;
pub use texture::{Texture, TextureChannel, TextureRegistry};

pub type FieldId = String;
