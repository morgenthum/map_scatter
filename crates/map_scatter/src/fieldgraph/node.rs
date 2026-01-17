//! Node specifications for the field graph.
//!
//! This module defines the data model for field nodes used by the field graph
//! subsystem. Each [`NodeSpec`] represents a typed operation in a DAG.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::texture::TextureChannel;
use crate::fieldgraph::FieldId;

/// Parameters for a constant value node.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ConstantParams {
    /// The constant value.
    pub value: f32,
}

/// Parameters for a texture sampling node.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct TextureParams {
    /// The ID of the texture to sample from.
    pub texture_id: String,
    /// The channel of the texture to sample.
    pub channel: TextureChannel,
}

/// Parameters for a clamp node.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ClampParams {
    /// Minimum value to clamp to.
    pub min: f32,
    /// Maximum value to clamp to.
    pub max: f32,
}

/// Parameters for a smoothstep node.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct SmoothStepParams {
    /// Lower edge of the transition.
    pub edge0: f32,
    /// Upper edge of the transition.
    pub edge1: f32,
}

/// Parameters for a scale node.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ScaleParams {
    /// Scaling factor.
    pub factor: f32,
}

/// Parameters for a power node.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct PowParams {
    /// Exponent value.
    pub exp: f32,
}

/// Parameters for an EDT normalize node.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct EdtNormalizeParams {
    /// Threshold value to avoid division by zero.
    pub threshold: f32,
    /// Maximum distance value for normalization.
    pub d_max: f32,
}

/// Specification of a node in the field graph.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub enum NodeSpec {
    Constant {
        /// Constant node parameters.
        params: ConstantParams,
    },
    Texture {
        /// Texture sampling parameters.
        params: TextureParams,
    },
    Add {
        /// Input field ids to sum.
        inputs: Vec<FieldId>,
    },
    Sub {
        /// Input field ids to subtract in order.
        inputs: Vec<FieldId>,
    },
    Mul {
        /// Input field ids to multiply.
        inputs: Vec<FieldId>,
    },
    Min {
        /// Input field ids to take the minimum of.
        inputs: Vec<FieldId>,
    },
    Max {
        /// Input field ids to take the maximum of.
        inputs: Vec<FieldId>,
    },
    Invert {
        /// Input field id to invert.
        inputs: Vec<FieldId>,
    },
    Scale {
        /// Input field ids to scale (first input used).
        inputs: Vec<FieldId>,
        /// Scale operation parameters.
        params: ScaleParams,
    },
    Clamp {
        /// Input field ids to clamp (first input used).
        inputs: Vec<FieldId>,
        /// Clamp operation parameters.
        params: ClampParams,
    },
    SmoothStep {
        /// Input field ids to smoothstep (first input used).
        inputs: Vec<FieldId>,
        /// Smoothstep operation parameters.
        params: SmoothStepParams,
    },
    Pow {
        /// Input field ids to exponentiate (first input used).
        inputs: Vec<FieldId>,
        /// Exponentiation parameters.
        params: PowParams,
    },
    EdtNormalize {
        /// Input field ids for EDT normalization (first input used).
        inputs: Vec<FieldId>,
        /// EDT normalization parameters.
        params: EdtNormalizeParams,
    },
}

impl NodeSpec {
    /// Returns the input field IDs for this node.
    pub fn inputs(&self) -> &[FieldId] {
        match self {
            NodeSpec::Add { inputs }
            | NodeSpec::Sub { inputs }
            | NodeSpec::Mul { inputs }
            | NodeSpec::Min { inputs }
            | NodeSpec::Max { inputs }
            | NodeSpec::Invert { inputs }
            | NodeSpec::Scale { inputs, .. }
            | NodeSpec::Clamp { inputs, .. }
            | NodeSpec::SmoothStep { inputs, .. }
            | NodeSpec::Pow { inputs, .. }
            | NodeSpec::EdtNormalize { inputs, .. } => inputs,
            NodeSpec::Constant { .. } | NodeSpec::Texture { .. } => &[],
        }
    }

    /// Creates a new constant value node specification.
    pub fn constant(value: f32) -> Self {
        NodeSpec::Constant {
            params: ConstantParams { value },
        }
    }

    /// Creates a new texture sampling node specification.
    pub fn texture(id: impl Into<String>, channel: TextureChannel) -> Self {
        NodeSpec::Texture {
            params: TextureParams {
                texture_id: id.into(),
                channel,
            },
        }
    }

    /// Creates a new addition node specification.
    pub fn add(inputs: Vec<FieldId>) -> Self {
        NodeSpec::Add { inputs }
    }

    /// Creates a new subtraction node specification.
    pub fn sub(inputs: Vec<FieldId>) -> Self {
        NodeSpec::Sub { inputs }
    }

    /// Creates a new multiplication node specification.
    pub fn mul(inputs: Vec<FieldId>) -> Self {
        NodeSpec::Mul { inputs }
    }

    /// Creates a new minimum node specification.
    pub fn min(inputs: Vec<FieldId>) -> Self {
        NodeSpec::Min { inputs }
    }

    /// Creates a new maximum node specification.
    pub fn max(inputs: Vec<FieldId>) -> Self {
        NodeSpec::Max { inputs }
    }

    /// Creates a new inversion node specification.
    pub fn invert(input: FieldId) -> Self {
        NodeSpec::Invert {
            inputs: vec![input],
        }
    }

    /// Creates a new scaling node specification.
    pub fn scale(input: FieldId, factor: f32) -> Self {
        NodeSpec::Scale {
            inputs: vec![input],
            params: ScaleParams { factor },
        }
    }

    /// Creates a new clamping node specification.
    pub fn clamp(input: FieldId, min: f32, max: f32) -> Self {
        NodeSpec::Clamp {
            inputs: vec![input],
            params: ClampParams { min, max },
        }
    }

    /// Creates a new smoothstep node specification.
    pub fn smoothstep(input: FieldId, edge0: f32, edge1: f32) -> Self {
        NodeSpec::SmoothStep {
            inputs: vec![input],
            params: SmoothStepParams { edge0, edge1 },
        }
    }

    /// Creates a new power node specification.
    pub fn pow(input: FieldId, exp: f32) -> Self {
        NodeSpec::Pow {
            inputs: vec![input],
            params: PowParams { exp },
        }
    }

    /// Creates a new EDT normalization node specification.
    pub fn edt_normalize(input: FieldId, threshold: f32, d_max: f32) -> Self {
        NodeSpec::EdtNormalize {
            inputs: vec![input],
            params: EdtNormalizeParams { threshold, d_max },
        }
    }
}
