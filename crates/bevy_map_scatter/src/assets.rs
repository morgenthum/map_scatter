use core::result::Result;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::tasks::ConditionalSendFuture;
use map_scatter::prelude::*;
use serde::{Deserialize, Serialize};

/// Asset describing a complete scatter [`Plan`] for `map_scatter`.
#[derive(Asset, TypePath, Clone, Debug, Serialize, Deserialize)]
pub struct ScatterPlanAsset {
    pub layers: Vec<ScatterLayerDef>,
}

/// Layer definition within a [`ScatterPlanAsset`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScatterLayerDef {
    pub id: String,
    pub kinds: Vec<ScatterKindDef>,
    pub sampling: SamplingDef,
    pub overlay_mask_size_px: Option<(u32, u32)>,
    pub overlay_brush_radius_px: Option<i32>,
    pub selection_strategy: SelectionStrategyDef,
}

/// Kind definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScatterKindDef {
    pub id: String,
    pub spec: FieldGraphSpec,
}

/// Selection strategy for layers.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SelectionStrategyDef {
    WeightedRandom,
    HighestProbability,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ParentDef {
    Count(usize),
    Density(f32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SamplingDef {
    UniformRandom {
        count: usize,
    },
    Halton {
        count: usize,
        bases: (u32, u32),
        start_index: u32,
        rotate: bool,
    },
    FibonacciLattice {
        count: usize,
        rotate: bool,
    },
    StratifiedMultiJitter {
        count: usize,
        rotate: bool,
    },
    BestCandidate {
        count: usize,
        k: usize,
    },
    PoissonDisk {
        radius: f32,
    },
    JitterGrid {
        jitter: f32,
        cell_size: f32,
    },
    HexJitterGrid {
        jitter: f32,
        cell_size: f32,
    },
    ClusteredThomas {
        parents: ParentDef,
        mean_children: f32,
        sigma: f32,
        clamp_inside: bool,
    },
    ClusteredNeymanScott {
        parents: ParentDef,
        mean_children: f32,
        radius: f32,
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
            let asset: ScatterPlanAsset =
                ron::de::from_bytes(&bytes).map_err(|e| anyhow::anyhow!(e))?;
            Ok(asset)
        })
    }
}

impl FromWorld for ScatterPlanAssetLoader {
    fn from_world(_: &mut World) -> Self {
        ScatterPlanAssetLoader
    }
}
