use core::result::Result;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::tasks::ConditionalSendFuture;
use map_scatter::prelude::*;
use serde::{Deserialize, Serialize};

/// Asset describing a complete scatter [`Plan`] for `map_scatter`.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Asset, TypePath, Clone, Debug)]
pub struct ScatterPlanAsset {
    /// Ordered list of layer definitions in the plan.
    pub layers: Vec<ScatterLayerDef>,
}

/// Layer definition within a [`ScatterPlanAsset`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ScatterLayerDef {
    /// Unique identifier for this layer.
    pub id: String,
    /// Kinds evaluated in this layer.
    pub kinds: Vec<ScatterKindDef>,
    /// Sampling strategy used for candidate generation.
    pub sampling: SamplingDef,
    /// Optional overlay mask size in pixels (width, height).
    pub overlay_mask_size_px: Option<(u32, u32)>,
    /// Optional overlay brush radius in pixels.
    pub overlay_brush_radius_px: Option<i32>,
    /// Strategy for selecting a kind when multiple are valid.
    pub selection_strategy: SelectionStrategyDef,
}

/// Kind definition.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ScatterKindDef {
    /// Unique identifier for this kind.
    pub id: String,
    /// Field graph specification for this kind.
    pub spec: FieldGraphSpec,
}

/// Selection strategy for layers.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug)]
pub enum SelectionStrategyDef {
    WeightedRandom,
    HighestProbability,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub enum ParentDef {
    Count(
        /// Number of parent centers to generate.
        usize,
    ),
    Density(
        /// Parent density per unit area.
        f32,
    ),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub enum SamplingDef {
    UniformRandom {
        /// Number of candidate points to generate.
        count: usize,
    },
    Halton {
        /// Number of candidate points to generate.
        count: usize,
        /// Bases for the 2D Halton sequence.
        bases: (u32, u32),
        /// Starting index in the Halton sequence.
        start_index: u32,
        /// Apply Cranley-Patterson rotation.
        rotate: bool,
    },
    FibonacciLattice {
        /// Number of candidate points to generate.
        count: usize,
        /// Apply Cranley-Patterson rotation.
        rotate: bool,
    },
    StratifiedMultiJitter {
        /// Number of candidate points to generate.
        count: usize,
        /// Apply Cranley-Patterson rotation.
        rotate: bool,
    },
    BestCandidate {
        /// Number of candidate points to generate.
        count: usize,
        /// Trials per point for best-candidate selection.
        k: usize,
    },
    PoissonDisk {
        /// Minimum distance between points in world units.
        radius: f32,
    },
    JitterGrid {
        /// Jitter amount in [0, 1].
        jitter: f32,
        /// Cell size for the base grid in world units.
        cell_size: f32,
    },
    HexJitterGrid {
        /// Jitter amount in [0, 1].
        jitter: f32,
        /// Base spacing along X in world units.
        cell_size: f32,
    },
    ClusteredThomas {
        /// Parent placement configuration.
        parents: ParentDef,
        /// Mean number of children per parent.
        mean_children: f32,
        /// Gaussian sigma for child offsets.
        sigma: f32,
        /// Clamp children inside the domain bounds.
        clamp_inside: bool,
    },
    ClusteredNeymanScott {
        /// Parent placement configuration.
        parents: ParentDef,
        /// Mean number of children per parent.
        mean_children: f32,
        /// Disk radius for child offsets.
        radius: f32,
        /// Clamp children inside the domain bounds.
        clamp_inside: bool,
    },
}

impl From<&ScatterKindDef> for Kind {
    fn from(value: &ScatterKindDef) -> Self {
        Kind::new(value.id.clone(), value.spec.clone())
    }
}

impl From<ScatterKindDef> for Kind {
    fn from(value: ScatterKindDef) -> Self {
        Kind::new(value.id, value.spec)
    }
}

impl From<SelectionStrategyDef> for SelectionStrategy {
    fn from(value: SelectionStrategyDef) -> Self {
        match value {
            SelectionStrategyDef::WeightedRandom => SelectionStrategy::WeightedRandom,
            SelectionStrategyDef::HighestProbability => SelectionStrategy::HighestProbability,
        }
    }
}

impl From<&ScatterLayerDef> for Layer {
    fn from(def: &ScatterLayerDef) -> Self {
        let kinds: Vec<Kind> = def.kinds.iter().map(|k| k.into()).collect();
        let sampling: Box<dyn PositionSampling> = sampling_runtime(&def.sampling);
        let mut layer = Layer::new(def.id.clone(), kinds, sampling);

        if let (Some(size), Some(radius)) = (
            def.overlay_mask_size_px.as_ref(),
            def.overlay_brush_radius_px,
        ) {
            layer = layer.with_overlay(*size, radius);
        }

        layer.with_selection_strategy(def.selection_strategy.into())
    }
}

impl From<&ScatterPlanAsset> for Plan {
    fn from(asset: &ScatterPlanAsset) -> Self {
        let layers: Vec<Layer> = asset.layers.iter().map(|l| l.into()).collect();
        Plan::new().with_layers(layers)
    }
}

impl From<ScatterPlanAsset> for Plan {
    fn from(asset: ScatterPlanAsset) -> Self {
        (&asset).into()
    }
}

/// Convert a `SamplingDef` into a boxed runtime sampler.
fn sampling_runtime(def: &SamplingDef) -> Box<dyn PositionSampling> {
    match def {
        SamplingDef::UniformRandom { count } => Box::new(UniformRandomSampling { count: *count }),
        SamplingDef::Halton {
            count,
            bases,
            start_index,
            rotate,
        } => Box::new(HaltonSampling {
            count: *count,
            bases: *bases,
            start_index: *start_index,
            rotate: *rotate,
        }),
        SamplingDef::FibonacciLattice { count, rotate } => Box::new(FibonacciLatticeSampling {
            count: *count,
            rotate: *rotate,
        }),
        SamplingDef::StratifiedMultiJitter { count, rotate } => {
            Box::new(StratifiedMultiJitterSampling {
                count: *count,
                rotate: *rotate,
            })
        }
        SamplingDef::BestCandidate { count, k } => Box::new(BestCandidateSampling {
            count: *count,
            k: *k,
        }),
        SamplingDef::PoissonDisk { radius } => Box::new(PoissonDiskSampling { radius: *radius }),
        SamplingDef::JitterGrid { jitter, cell_size } => {
            Box::new(JitterGridSampling::new(*jitter, *cell_size))
        }
        SamplingDef::HexJitterGrid { jitter, cell_size } => {
            Box::new(HexJitterGridSampling::new(*jitter, *cell_size))
        }
        SamplingDef::ClusteredThomas {
            parents,
            mean_children,
            sigma,
            clamp_inside,
        } => {
            let base = match parents {
                ParentDef::Count(n) => {
                    ClusteredSampling::thomas_with_count(*n, *mean_children, *sigma)
                }
                ParentDef::Density(d) => {
                    ClusteredSampling::thomas_with_density(*d, *mean_children, *sigma)
                }
            };
            Box::new(base.with_clamp_inside(*clamp_inside))
        }
        SamplingDef::ClusteredNeymanScott {
            parents,
            mean_children,
            radius,
            clamp_inside,
        } => {
            let base = match parents {
                ParentDef::Count(n) => {
                    ClusteredSampling::neyman_scott_with_count(*n, *mean_children, *radius)
                }
                ParentDef::Density(d) => {
                    ClusteredSampling::neyman_scott_with_density(*d, *mean_children, *radius)
                }
            };
            Box::new(base.with_clamp_inside(*clamp_inside))
        }
    }
}

/// Asset loader for [`ScatterPlanAsset`] using RON files with `.scatter` extension.
#[derive(TypePath)]
pub struct ScatterPlanAssetLoader;

impl AssetLoader for ScatterPlanAssetLoader {
    type Asset = ScatterPlanAsset;
    type Settings = ();
    type Error = anyhow::Error;

    fn extensions(&self) -> &[&str] {
        &["scatter"]
    }

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _context: &mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            #[cfg(feature = "ron")]
            {
                let asset: ScatterPlanAsset =
                    ron::de::from_bytes(&bytes).map_err(|e| anyhow::anyhow!(e))?;
                Ok(asset)
            }
            #[cfg(not(feature = "ron"))]
            {
                let _ = bytes;
                Err(anyhow::anyhow!(
                    "bevy_map_scatter: enable the `ron` feature to load .scatter assets"
                ))
            }
        })
    }
}

impl FromWorld for ScatterPlanAssetLoader {
    fn from_world(_: &mut World) -> Self {
        ScatterPlanAssetLoader
    }
}
