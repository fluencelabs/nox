use fluence_libp2p::types::Outlet;
use fluence_libp2p::PeerId;
use std::collections::HashSet;
use std::time::Duration;
use thiserror::Error;

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
    pub events: HashSet<PeerEventType>,
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
pub enum PeerEvent {
    Connect { peer_id: PeerId },
    Disconnect { peer_id: PeerId },
}

impl PeerEvent {
    pub(crate) fn get_type(&self) -> PeerEventType {
        match self {
            PeerEvent::Connect { .. } => PeerEventType::Connect,
            PeerEvent::Disconnect { .. } => PeerEventType::Disconnect,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum PeerEventType {
    Connect,
    Disconnect,
}

pub enum Command {
    /// Subscribe a listener with a specified ID to a list of events
    Add(Listener, ListenerConfig),
    /// Remove all listeners with this ID
    Remove(ListenerId),
}

#[derive(Error, Debug)]
pub enum DispatcherError {
    #[error("can't send a message to the scheduler")]
    CommandSendError,
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
            .map_err(|_| DispatcherError::CommandSendError)
    }

    pub fn add(&self, listener: Listener, config: ListenerConfig) -> Result<(), DispatcherError> {
        self.send(Command::Add(listener, config))
    }

    pub fn remove(&self, id: ListenerId) -> Result<(), DispatcherError> {
        self.send(Command::Remove(id))
    }
}
