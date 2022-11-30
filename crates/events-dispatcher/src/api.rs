use fluence_libp2p::types::{OneshotOutlet, Outlet};
use futures::{channel::oneshot, future::BoxFuture, FutureExt};
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;
use connection_pool::LifecycleEvent;

#[derive(Debug)]
pub struct TimerConfig {
    pub period: Duration,
}

#[derive(Debug)]
pub enum EventConfig {
    Timer(TimerConfig),
    PeerEvent(PeerEventConfig),
}

#[derive(Debug)]
pub struct PeerEventConfig {
    pub events: Vec<PeerEventType>,
}

#[derive(Debug)]
pub struct ListenerConfig {
    pub configs: Vec<EventConfig>,
}

pub type ListenerId = String;

#[derive(Debug)]
pub struct Listener {
    pub id: ListenerId,
}

#[derive(Debug)]
pub struct ListenerEvent {
    pub id: ListenerId,
    pub event: Event,
}

#[derive(Clone, Debug)]
pub enum Event {
    Timer,
    Peer(PeerEvent),
}

#[derive(Clone, Debug)]
pub struct PeerEvent(pub LifecycleEvent);

impl PeerEvent {
    pub(crate) fn get_type(&self) -> PeerEventType {
        match self.0 {
            LifecycleEvent::Connected { .. } => PeerEventType::Connected,
            LifecycleEvent::Disconnected { .. } => PeerEventType::Disconnected,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Deserialize)]
pub enum PeerEventType {
    Connected,
    Disconnected,
}

pub enum Command {
    /// Subscribe a listener with a specified ID to a list of events
    Add(Listener, ListenerConfig),
    /// Remove all listeners with this ID
    Remove(ListenerId, OneshotOutlet<()>),
}

#[derive(Error, Debug)]
pub enum DispatcherError {
    #[error("can't send a message to the dispatcher")]
    SendError,
    #[error("can't receive a message from the dispatcher")]
    ReplyError,
}

pub struct EventsDispatcherApi {
    pub(crate) send_cmd_channel: Outlet<Command>,
}

impl std::fmt::Debug for EventsDispatcherApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventDispatcherApi").finish()
    }
}

impl EventsDispatcherApi {
    fn send(&self, cmd: Command) -> Result<(), DispatcherError> {
        self.send_cmd_channel
            .unbounded_send(cmd)
            .map_err(|_| DispatcherError::SendError)
    }

    pub fn add(&self, listener: Listener, config: ListenerConfig) -> Result<(), DispatcherError> {
        self.send(Command::Add(listener, config))
    }

    pub fn remove(&self, id: ListenerId) -> BoxFuture<'static, Result<(), DispatcherError>> {
        let (send, recv) = oneshot::channel();
        if let Err(err) = self.send(Command::Remove(id, send)) {
            return futures::future::err(err).boxed();
        }
        recv.map(|r| r.map_err(|_| DispatcherError::ReplyError))
            .boxed()
    }
}
