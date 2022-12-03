use fluence_libp2p::types::{OneshotOutlet, Outlet};
use futures::{channel::oneshot, future::BoxFuture, FutureExt};
use std::time::Duration;
use thiserror::Error;
use connection_pool::LifecycleEvent;

#[derive(Debug)]
pub struct TriggersConfig {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spell {
    pub id: SpellId,
}

#[derive(Debug)]
pub struct SpellEvent {
    pub id: SpellId,
    pub event: Event,
}

#[derive(Clone, Debug)]
pub enum Event {
    // Event is triggered by timer.
    Timer,
    // Event is triggered by peer event.
    Peer(PeerEvent),
}

#[derive(Clone, Debug)]
pub enum PeerEvent {
    // Event is triggered by connection pool event
    ConnectionPool(LifecycleEvent),
}

impl PeerEvent {
    pub(crate) fn get_type(&self) -> PeerEventType {
        match self {
            PeerEvent::ConnectionPool(LifecycleEvent::Connected { .. }) => PeerEventType::Connected,
            PeerEvent::ConnectionPool(LifecycleEvent::Disconnected { .. }) => PeerEventType::Disconnected,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum PeerEventType {
    Connected,
    Disconnected,
}

pub enum Command {
    /// Subscribe a listener with a specified ID to a list of events
    Subscribe(Spell, TriggersConfig),
    /// Remove all listeners with this ID
    Unsubscribe(SpellId, OneshotOutlet<()>),
}

#[derive(Error, Debug)]
pub enum DispatcherError {
    #[error("can't send a message to the dispatcher")]
    SendError,
    #[error("can't receive a message from the dispatcher")]
    ReplyError,
}

pub struct SpellEventBusApi {
    pub(crate) send_cmd_channel: Outlet<Command>,
}

impl std::fmt::Debug for SpellEventBusApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpellEventBusApi").finish()
    }
}

impl SpellEventBusApi {
    fn send(&self, cmd: Command) -> Result<(), DispatcherError> {
        self.send_cmd_channel
            .unbounded_send(cmd)
            .map_err(|_| DispatcherError::SendError)
    }

    pub fn subscribe(&self, spell: Spell, config: TriggersConfig) -> Result<(), DispatcherError> {
        self.send(Command::Subscribe(spell, config))
    }

    pub fn unsubscribe(&self, id: SpellId) -> BoxFuture<'static, Result<(), DispatcherError>> {
        let (send, recv) = oneshot::channel();
        if let Err(err) = self.send(Command::Unsubscribe(id, send)) {
            return futures::future::err(err).boxed();
        }
        recv.map(|r| r.map_err(|_| DispatcherError::ReplyError))
            .boxed()
    }
}
