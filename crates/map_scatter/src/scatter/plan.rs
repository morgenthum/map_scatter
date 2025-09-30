//! Planning module for defining scatter layers and plans.
use crate::sampling::PositionSampling;
use crate::scatter::Kind;

/// Strategy for selecting a kind when multiple are placeable at a candidate position.
#[derive(Clone, Copy, Debug)]
pub enum SelectionStrategy {
    WeightedRandom,
    HighestProbability,
}

/// A layer in a scatter plan.
#[non_exhaustive]
pub struct Layer {
    /// Unique identifier for this layer.
    pub id: String,
    /// Kinds to consider for placement in this layer.
    pub kinds: Vec<Kind>,
    /// Sampling strategy to generate candidate positions.
    pub sampling: Box<dyn PositionSampling>,
    /// Optional overlay mask size in pixels (width, height).
    pub overlay_mask_size_px: Option<(u32, u32)>,
    /// Optional overlay brush radius in pixels.
    pub overlay_brush_radius_px: Option<i32>,
    /// Strategy for selecting a kind.
    pub selection_strategy: SelectionStrategy,
}

impl Layer {
    /// Create a new layer with required fields.
    pub fn new(
        id: impl Into<String>,
        kinds: Vec<Kind>,
        sampling: Box<dyn PositionSampling>,
    ) -> Self {
        Self {
            id: id.into(),
            kinds,
            sampling,
            overlay_mask_size_px: None,
            overlay_brush_radius_px: None,
            selection_strategy: SelectionStrategy::WeightedRandom,
        }
    }

    /// Create a new layer with required fields and a concrete sampling strategy.
    pub fn new_with<S: PositionSampling + 'static>(
        id: impl Into<String>,
        kinds: Vec<Kind>,
        sampling: S,
    ) -> Self {
        Self::new(id, kinds, Box::new(sampling))
    }

    /// Set both optional overlay mask size in pixels (width, height) and brush radius in pixels.
    pub fn with_overlay(mut self, size: (u32, u32), radius: i32) -> Self {
        self.overlay_mask_size_px = Some(size);
        self.overlay_brush_radius_px = Some(radius);
        self
    }

    /// Set the selection strategy for this layer.
    pub fn with_selection_strategy(mut self, strategy: SelectionStrategy) -> Self {
        self.selection_strategy = strategy;
        self
    }
}

/// A scatter plan composed of one or more [`Layer`]s.
#[derive(Default)]
#[non_exhaustive]
pub struct Plan {
    pub layers: Vec<Layer>,
}

impl Plan {
    /// Create a new empty plan.
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Add a single layer to the plan.
    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layers.push(layer);
        self
    }

    /// Add multiple layers to the plan.
    pub fn with_layers(mut self, layers: Vec<Layer>) -> Self {
        self.layers.extend(layers);
        self
    }
}

#[cfg(test)]
mod tests {
    use mint::Vector2;
    use rand::RngCore;

    use super::*;
    use crate::prelude::FieldGraphSpec;

    fn kind(id: &str) -> Kind {
        Kind::new(id, FieldGraphSpec::default())
    }

    #[test]
    fn layer_builder_sets_optional_fields() {
        let layer = Layer::new_with("id", vec![kind("a")], JitterSampling {})
            .with_overlay((32, 16), 4)
            .with_selection_strategy(SelectionStrategy::HighestProbability);

        assert_eq!(layer.id, "id");
        assert_eq!(layer.kinds.len(), 1);
        assert_eq!(layer.overlay_mask_size_px, Some((32, 16)));
        assert_eq!(layer.overlay_brush_radius_px, Some(4));
        matches!(
            layer.selection_strategy,
            SelectionStrategy::HighestProbability
        )
        .then_some(())
        .expect("selection strategy set");
    }

    #[test]
    fn plan_builder_pushes_layers() {
        let layer = Layer::new("layer", vec![kind("a")], Box::new(JitterSampling {}));
        let plan = Plan::new().with_layer(layer);
        assert_eq!(plan.layers.len(), 1);
    }

    struct JitterSampling;

    impl PositionSampling for JitterSampling {
        fn generate(
            &self,
            _domain_extent: Vector2<f32>,
            _rng: &mut dyn RngCore,
        ) -> Vec<Vector2<f32>> {
            Vec::new()
        }
    }
}
