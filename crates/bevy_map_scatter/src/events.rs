use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use map_scatter::prelude::{EventSink, ScatterEvent, ScatterEventKind};

/// Delivery priority for scatter events in the bus.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScatterEventPriority {
    High,
    Low,
}

/// Filters which scatter events are forwarded onto the bus.
#[derive(Debug, Clone)]
pub struct ScatterEventFilter {
    /// Emit run start/finish events.
    pub emit_run_events: bool,
    /// Emit layer start/finish events.
    pub emit_layer_events: bool,
    /// Emit per-position evaluation events.
    pub emit_position_evaluated: bool,
    /// Emit per-placement events.
    pub emit_placement_made: bool,
    /// Emit overlay generation events.
    pub emit_overlay_generated: bool,
    /// Emit warnings.
    pub emit_warnings: bool,
}

impl ScatterEventFilter {
    /// High-level events only (no per-position or per-placement spam).
    pub fn high_level() -> Self {
        Self {
            emit_run_events: true,
            emit_layer_events: true,
            emit_position_evaluated: false,
            emit_placement_made: false,
            emit_overlay_generated: true,
            emit_warnings: true,
        }
    }

    /// Emit all events.
    pub fn verbose() -> Self {
        Self {
            emit_run_events: true,
            emit_layer_events: true,
            emit_position_evaluated: true,
            emit_placement_made: true,
            emit_overlay_generated: true,
            emit_warnings: true,
        }
    }

    pub fn wants(&self, kind: ScatterEventKind) -> bool {
        match kind {
            ScatterEventKind::RunStarted | ScatterEventKind::RunFinished => self.emit_run_events,
            ScatterEventKind::LayerStarted | ScatterEventKind::LayerFinished => {
                self.emit_layer_events
            }
            ScatterEventKind::PositionEvaluated => self.emit_position_evaluated,
            ScatterEventKind::PlacementMade => self.emit_placement_made,
            ScatterEventKind::OverlayGenerated => self.emit_overlay_generated,
            ScatterEventKind::Warning => self.emit_warnings,
        }
    }

    pub fn priority(kind: ScatterEventKind) -> ScatterEventPriority {
        match kind {
            ScatterEventKind::PositionEvaluated | ScatterEventKind::PlacementMade => {
                ScatterEventPriority::Low
            }
            _ => ScatterEventPriority::High,
        }
    }
}

impl Default for ScatterEventFilter {
    fn default() -> Self {
        Self::high_level()
    }
}

/// Configuration for the scatter bus.
#[derive(Resource, Debug, Clone)]
pub struct ScatterBusConfig {
    /// Maximum number of messages buffered before backpressure applies.
    pub capacity: usize,
    /// Event filtering rules.
    pub filter: ScatterEventFilter,
}

impl ScatterBusConfig {
    pub fn new(capacity: usize, filter: ScatterEventFilter) -> Self {
        Self { capacity, filter }
    }

    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    pub fn with_filter(mut self, filter: ScatterEventFilter) -> Self {
        self.filter = filter;
        self
    }
}

impl Default for ScatterBusConfig {
    fn default() -> Self {
        Self {
            capacity: 1024,
            filter: ScatterEventFilter::high_level(),
        }
    }
}

/// Bevy message containing the originating scatter request entity and the underlying [`ScatterEvent`].
#[non_exhaustive]
#[derive(Message, Debug, Clone)]
pub struct ScatterMessage {
    /// Entity that initiated the scatter request.
    pub request_entity: Entity,
    /// Scatter event emitted by the runner.
    pub event: ScatterEvent,
}

/// Global bus for streaming scatter events from async tasks to the main thread.
#[derive(Resource, Clone)]
pub struct ScatterBus {
    /// Sender used by async tasks to publish scatter messages.
    tx: Sender<ScatterMessage>,
    /// Receiver drained on the main thread to dispatch messages.
    rx: Receiver<ScatterMessage>,
    config: ScatterBusConfig,
}

impl ScatterBus {
    pub fn new(config: ScatterBusConfig) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(config.capacity);
        Self { tx, rx, config }
    }

    pub fn sender(&self) -> &Sender<ScatterMessage> {
        &self.tx
    }

    pub fn receiver(&self) -> &Receiver<ScatterMessage> {
        &self.rx
    }

    pub fn config(&self) -> &ScatterBusConfig {
        &self.config
    }

    pub fn filter(&self) -> &ScatterEventFilter {
        &self.config.filter
    }
}

impl FromWorld for ScatterBus {
    fn from_world(world: &mut World) -> Self {
        let config = world
            .get_resource::<ScatterBusConfig>()
            .cloned()
            .unwrap_or_default();
        Self::new(config)
    }
}

/// Event sink that forwards events to the global scatter bus, tagging each event with the request entity.
pub struct ChannelSink {
    /// Request entity associated with the emitted events.
    pub request: Entity,
    /// Sender used to forward messages onto the bus.
    pub tx: Sender<ScatterMessage>,
    /// Filter applied to outgoing events.
    pub filter: ScatterEventFilter,
}

impl EventSink for ChannelSink {
    #[inline]
    fn send(&mut self, event: ScatterEvent) {
        let kind = event.kind();
        if !self.filter.wants(kind) {
            return;
        }
        let message = ScatterMessage {
            request_entity: self.request,
            event,
        };
        match ScatterEventFilter::priority(kind) {
            ScatterEventPriority::High => {
                let _ = self.tx.send(message);
            }
            ScatterEventPriority::Low => {
                let _ = self.tx.try_send(message);
            }
        }
    }

    fn wants(&self, kind: ScatterEventKind) -> bool {
        self.filter.wants(kind)
    }
}
