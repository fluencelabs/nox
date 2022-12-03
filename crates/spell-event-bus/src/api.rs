use connection_pool::LifecycleEvent;
use fluence_libp2p::types::{OneshotOutlet, Outlet};
use futures::{channel::oneshot, future::BoxFuture, FutureExt};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug)]
pub struct SpellTriggerConfigs {
    pub triggers: Vec<TriggerConfig>,
}

#[derive(Debug)]
pub enum TriggerConfig {
    Timer(TimerConfig),
    PeerEvent(PeerEventConfig),
}

#[derive(Debug)]
pub struct TimerConfig {
    pub period: Duration,
}

#[derive(Debug)]
pub struct PeerEventConfig {
    pub events: Vec<PeerEventType>,
}

pub type SpellId = String;

#[derive(Debug)]
pub struct SpellEvent {
    pub id: SpellId,
    pub event: Event,
}

#[derive(Clone, Debug)]
pub enum Event {
    /// Event is triggered by timer.
    Timer,
    /// Event is triggered by a peer event.
    Peer(PeerEvent),
}

#[derive(Clone, Debug)]
pub enum PeerEvent {
    /// Event is triggered by connection pool event
    ConnectionPool(LifecycleEvent),
}

impl PeerEvent {
    pub(crate) fn get_type(&self) -> PeerEventType {
        match self {
            PeerEvent::ConnectionPool(LifecycleEvent::Connected { .. }) => PeerEventType::Connected,
            PeerEvent::ConnectionPool(LifecycleEvent::Disconnected { .. }) => {
                PeerEventType::Disconnected
            }
        }
    }
}

/// Types of events that are available for subscription.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum PeerEventType {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub(crate) enum Command {
    /// Subscribe a listener with a specified ID to a list of events
    Subscribe(SpellId, SpellTriggerConfigs),
    /// Remove all listeners with this ID
    Unsubscribe(SpellId, OneshotOutlet<()>),
}

#[derive(Error, Debug)]
pub enum EventBusError {
    #[error("can't send a command `{command}` for spell `{id}` to spell-event-bus: {reason}")]
    SendError {
        id: SpellId,
        command: String,
        reason: String,
    },
    #[error("can't receive a message from the bus on behalf of spell {0}")]
    ReplyError(SpellId),
}

#[derive(Clone)]
pub struct SpellEventBusApi {
    pub(crate) send_cmd_channel: Outlet<Command>,
}

impl std::fmt::Debug for SpellEventBusApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpellEventBusApi").finish()
    }
}

impl SpellEventBusApi {
    fn send(&self, cmd: Command) -> Result<(), EventBusError> {
        self.send_cmd_channel.unbounded_send(cmd).map_err(|e| {
            let reason = if e.is_disconnected() {
                "disconnected"
            } else {
                "full"
            }
            .to_string();
            let command = e.into_inner();
            let (id, command) = match command {
                Command::Subscribe(id, _) => (id, "subscribe".to_string()),
                Command::Unsubscribe(id, _) => (id, "unsubscribe".to_string()),
            };
            EventBusError::SendError {
                id,
                command,
                reason,
            }
        })
    }

    /// Subscribe a spell to a list of events
    /// The spell can be subscribed multiple times to different events, but to only one timer.
    /// Note that multiple subscriptions to the same event will result in multiple events of the same type being sent.
    pub fn subscribe(
        &self,
        spell_id: SpellId,
        config: SpellTriggerConfigs,
    ) -> Result<(), EventBusError> {
        self.send(Command::Subscribe(spell_id, config))
    }

    /// Unsubscribe a spell from all events.
    pub fn unsubscribe(&self, id: SpellId) -> BoxFuture<'static, Result<(), EventBusError>> {
        let (send, recv) = oneshot::channel();
        if let Err(err) = self.send(Command::Unsubscribe(id.clone(), send)) {
            return futures::future::err(err).boxed();
        }
        recv.map(|r| r.map_err(|_| EventBusError::ReplyError(id)))
            .boxed()
    }
}
