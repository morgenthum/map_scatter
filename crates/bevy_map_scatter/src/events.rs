use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use map_scatter::prelude::{EventSink, ScatterEvent};

/// Bevy message containing the originating scatter request entity and the underlying [`ScatterEvent`].
#[derive(Message, Debug, Clone)]
pub struct ScatterMessage {
    /// Entity that initiated the scatter request.
    pub request_entity: Entity,
    /// Scatter event emitted by the runner.
    pub event: ScatterEvent,
}

/// Global bus for streaming scatter events from async tasks to the main thread.
#[derive(Resource)]
pub struct ScatterBus {
    /// Sender used by async tasks to publish scatter messages.
    pub tx: Sender<ScatterMessage>,
    /// Receiver drained on the main thread to dispatch messages.
    pub rx: Receiver<ScatterMessage>,
}

impl Default for ScatterBus {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self { tx, rx }
    }
}

/// Event sink that forwards events to the global scatter bus, tagging each event with the request entity.
pub struct ChannelSink {
    /// Request entity associated with the emitted events.
    pub request: Entity,
    /// Sender used to forward messages onto the bus.
    pub tx: Sender<ScatterMessage>,
}

impl EventSink for ChannelSink {
    #[inline]
    fn send(&mut self, event: ScatterEvent) {
        let _ = self.tx.send(ScatterMessage {
            request_entity: self.request,
            event,
        });
    }
}
