//! Event types and sinks for observing scatter runs.
//!
//! This module defines [`ScatterEvent`] and a set of sinks and adapters to emit,
//! collect, or forward events while executing a [`crate::scatter::plan::Plan`]
//! via [`crate::scatter::runner::ScatterRunner`], [`crate::scatter::runner::run_plan`],
//! or [`crate::scatter::runner::run_layer`].
use glam::Vec2;

use crate::scatter::runner::{Placement, RunConfig, RunResult};
use crate::scatter::KindId;

/// Describes events emitted by scatter operations.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ScatterEvent {
    /// Emitted when a run starts for a plan.
    RunStarted {
        /// The run configuration used.
        config: RunConfig,
        /// Number of layers in the plan.
        layer_count: usize,
    },

    /// Emitted when the entire plan finishes.
    RunFinished {
        /// Aggregated result for all layers.
        result: RunResult,
    },

    /// Emitted when a layer starts processing.
    LayerStarted {
        /// Index of the layer in the plan.
        index: usize,
        /// The layer id.
        id: String,
        /// The kinds configured on this layer.
        kinds: Vec<KindId>,
        /// If present, overlay mask (width, height) in pixels.
        overlay_mask_size_px: Option<(u32, u32)>,
        /// If present, overlay brush radius in pixels.
        overlay_brush_radius_px: Option<i32>,
    },

    /// Emitted when a layer finishes processing.
    LayerFinished {
        /// Index of the layer in the plan.
        index: usize,
        /// The layer id.
        id: String,
        /// Summary of what was evaluated and placed in this layer.
        result: RunResult,
        /// Name and size of a generated overlay, if any.
        overlay: Option<OverlaySummary>,
    },

    /// Emitted after a candidate position was evaluated for all kinds.
    PositionEvaluated {
        /// Index of the layer being processed.
        layer_index: usize,
        /// Id of the layer being processed.
        layer_id: String,
        /// Candidate position in domain coordinates.
        position: Vec2,
        /// Per-kind evaluation outcome at this position.
        evaluations: Vec<KindEvaluationLite>,
        /// Maximum weight over allowed kinds at this position.
        max_weight: f32,
    },

    /// Emitted when a placement is made.
    PlacementMade {
        /// Index of the layer that produced the placement.
        layer_index: usize,
        /// Id of the layer that produced the placement.
        layer_id: String,
        /// The placement data.
        placement: Placement,
    },

    /// Emitted when an overlay mask was generated for a layer.
    OverlayGenerated {
        /// Index of the layer in the plan.
        layer_index: usize,
        /// The layer id.
        layer_id: String,
        /// Overlay summary.
        summary: OverlaySummary,
    },

    /// Non-fatal warning generated during scatter.
    Warning {
        /// Context string (e.g. layer id, kind id).
        context: String,
        /// Human-readable message.
        message: String,
    },
}

/// Lightweight evaluation summary for a single kind at a position.
#[derive(Debug, Clone)]
pub struct KindEvaluationLite {
    /// Kind identifier for this evaluation.
    pub kind_id: KindId,
    /// Whether all gate fields passed for this position.
    pub allowed: bool,
    /// Final selection weight in [0, 1].
    pub weight: f32,
}

impl KindEvaluationLite {
    pub fn new(kind_id: impl Into<KindId>, allowed: bool, weight: f32) -> Self {
        Self {
            kind_id: kind_id.into(),
            allowed,
            weight,
        }
    }
}

/// Summary of an overlay mask for a layer.
#[derive(Debug, Clone)]
pub struct OverlaySummary {
    /// Overlay texture registry name.
    pub name: String,
    /// Pixel dimensions (width, height).
    pub size_px: (u32, u32),
}

impl OverlaySummary {
    pub fn new(name: impl Into<String>, size_px: (u32, u32)) -> Self {
        Self {
            name: name.into(),
            size_px,
        }
    }
}

/// A generic event sink that accepts [`ScatterEvent`]s.
pub trait EventSink {
    fn send(&mut self, event: ScatterEvent);

    fn send_many<I>(&mut self, events: I)
    where
        Self: Sized,
        I: IntoIterator<Item = ScatterEvent>,
    {
        for e in events {
            self.send(e);
        }
    }
}

/// A no-op event sink.
impl EventSink for () {
    #[inline]
    fn send(&mut self, _event: ScatterEvent) {}
}

/// An event sink that forwards to a user-provided closure.
pub struct FnSink<F>
where
    F: FnMut(ScatterEvent),
{
    f: F,
}

impl<F> FnSink<F>
where
    F: FnMut(ScatterEvent),
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> EventSink for FnSink<F>
where
    F: FnMut(ScatterEvent),
{
    #[inline]
    fn send(&mut self, event: ScatterEvent) {
        (self.f)(event);
    }
}

/// An event sink that collects all events in a `Vec`.
#[derive(Default)]
pub struct VecSink {
    events: Vec<ScatterEvent>,
}

impl VecSink {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            events: Vec::with_capacity(cap),
        }
    }

    pub fn into_inner(self) -> Vec<ScatterEvent> {
        self.events
    }

    pub fn as_slice(&self) -> &[ScatterEvent] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl EventSink for VecSink {
    #[inline]
    fn send(&mut self, event: ScatterEvent) {
        self.events.push(event);
    }
}

/// Fan-out sink that forwards each event to all contained sinks.
pub struct MultiSink<S: EventSink> {
    pub(crate) sinks: Vec<S>,
}

impl<S: EventSink> MultiSink<S> {
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    pub fn with_sinks(sinks: Vec<S>) -> Self {
        Self { sinks }
    }

    pub fn push(&mut self, sink: S) {
        self.sinks.push(sink);
    }

    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.sinks.len()
    }
}

impl<S: EventSink> Default for MultiSink<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: EventSink> EventSink for MultiSink<S> {
    fn send(&mut self, event: ScatterEvent) {
        if self.sinks.is_empty() {
            return;
        }
        let last_idx = self.sinks.len() - 1;
        for i in 0..last_idx {
            self.sinks[i].send(event.clone());
        }
        self.sinks[last_idx].send(event);
    }
}

/// Minimal adapter trait for types that can expose an [`EventSink`].
pub trait AsEventSink {
    fn as_event_sink(&mut self) -> &mut dyn EventSink;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_evaluation_lite_constructor_sets_fields() {
        let eval = KindEvaluationLite::new("tree", true, 0.8);
        assert_eq!(eval.kind_id, "tree");
        assert!(eval.allowed);
        assert_eq!(eval.weight, 0.8);
    }

    #[test]
    fn overlay_summary_constructor_sets_fields() {
        let summary = OverlaySummary::new("mask", (4, 4));
        assert_eq!(summary.name, "mask");
        assert_eq!(summary.size_px, (4, 4));
    }

    #[test]
    fn vec_sink_collects_events() {
        let mut sink = VecSink::with_capacity(2);
        assert!(sink.is_empty());
        sink.send(ScatterEvent::Warning {
            context: "a".into(),
            message: "m".into(),
        });
        sink.send(ScatterEvent::Warning {
            context: "b".into(),
            message: "n".into(),
        });
        assert_eq!(sink.len(), 2);
        sink.clear();
        assert!(sink.is_empty());
    }

    #[test]
    fn multi_sink_fans_out_events() {
        let sink_a = VecSink::new();
        let sink_b = VecSink::new();
        let mut multi = MultiSink::with_sinks(vec![sink_a, sink_b]);
        let event = ScatterEvent::Warning {
            context: "ctx".into(),
            message: "msg".into(),
        };
        multi.send(event.clone());
        assert_eq!(multi.sinks.len(), 2);
        assert_eq!(multi.sinks[0].len(), 1);
        assert_eq!(multi.sinks[1].len(), 1);
        // Ensure event clone happened correctly
        matches!(multi.sinks[0].as_slice()[0], ScatterEvent::Warning { .. })
            .then_some(())
            .expect("event captured");
    }

    #[test]
    fn fn_sink_invokes_callback() {
        let mut count = 0;
        let mut sink = FnSink::new(|_event| {
            count += 1;
        });
        sink.send(ScatterEvent::Warning {
            context: "ctx".into(),
            message: "msg".into(),
        });
        assert_eq!(count, 1);
    }
}
