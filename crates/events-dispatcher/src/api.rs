use fluence_libp2p::types::Outlet;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub enum Event {
    Connect { who: String },
    Disconnect { who: String },
}

impl Event {
    pub(crate) fn get_type(&self) -> EventType {
        match self {
            Event::Connect { .. } => EventType::Connect,
            Event::Disconnect { .. } => EventType::Disconnect,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum EventType {
    Connect,
    Disconnect,
}

pub enum Command {
    /// Subscribe a listener with a specified ID to a set of events
    /// with a handler for this specific group of events.
    Add(Listener<Event>, ListenerConfig),
    /// Remove all listeners with this ID
    Remove(ListenerId),
}

pub struct EventsDispatcherApi {
    pub(crate) send_channel: Outlet<Command>,
}

impl EventsDispatcherApi {
    pub fn send(&self, cmd: Command) {
        self.send_channel.unbounded_send(cmd).unwrap();
    }
}

pub type ListenerId = String;

pub struct ListenerConfig {
    pub events: HashSet<EventType>,
}

pub struct Listener<T> {
    pub id: ListenerId,
    pub callback: Box<dyn Fn(&T) + Send + Sync>,
}

impl<T> Listener<T> {
    pub(crate) fn notify(&self, event: &T) {
        (self.callback)(event);
    }
}
