use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use map_scatter::prelude::{EventSink, ScatterEvent};

/// Bevy message containing the originating scatter request entity and the underlying [`ScatterEvent`].
#[derive(Message, Debug, Clone)]
pub struct ScatterMessage {
    pub request_entity: Entity,
    pub event: ScatterEvent,
}

/// Global bus for streaming scatter events from async tasks to the main thread.
#[derive(Resource)]
pub struct ScatterBus {
    pub tx: Sender<ScatterMessage>,
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
    pub request: Entity,
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
