use connection_pool::LifecycleEvent;
use fluence_libp2p::types::{OneshotOutlet, Outlet};
use futures::channel::mpsc::SendError;
use futures::{channel::oneshot, future::BoxFuture, FutureExt};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SpellTriggerConfigs {
    pub triggers: Vec<TriggerConfig>,
}

#[derive(Debug, Clone)]
pub enum TriggerConfig {
    Timer(TimerConfig),
    PeerEvent(PeerEventConfig),
}

#[derive(Debug, Clone)]
pub struct TimerConfig {
    pub period: Duration,
}

#[derive(Debug, Clone)]
pub struct PeerEventConfig {
    pub events: Vec<PeerEventType>,
}

pub type SpellId = String;

#[derive(Debug)]
pub struct TriggerEvent {
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
pub(crate) struct Command {
    pub(crate) spell_id: SpellId,
    pub(crate) action: Action,
    pub(crate) reply: OneshotOutlet<()>,
}

#[derive(Debug, Clone)]
pub enum Action {
    /// Subscribe a spell to a list of triggers
    Subscribe(SpellTriggerConfigs),
    /// Remove all subscriptions of a spell
    Unsubscribe,
}

#[derive(Error, Debug)]
pub enum EventBusError {
    #[error(
        "can't send a command `{action:?}` for spell `{spell_id}` to spell-event-bus: {reason}"
    )]
    SendError {
        spell_id: SpellId,
        action: Action,
        reason: SendError,
    },
    #[error("can't receive a message from the bus on behalf of spell {0}: sending end is probably dropped")]
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
    fn send(
        &self,
        spell_id: SpellId,
        action: Action,
    ) -> BoxFuture<'static, Result<(), EventBusError>> {
        let (send, recv) = oneshot::channel();
        let command = Command {
            spell_id: spell_id.clone(),
            action: action.clone(),
            reply: send,
        };
        let result =
            self.send_cmd_channel
                .unbounded_send(command)
                .map_err(|e| EventBusError::SendError {
                    spell_id: spell_id.clone(),
                    action,
                    reason: e.into_send_error(),
                });

        if let Err(err) = result {
            return futures::future::err(err).boxed();
        }
        recv.map(|r| r.map_err(|_| EventBusError::ReplyError(spell_id)))
            .boxed()
    }

    /// Subscribe a spell to a list of events
    /// The spell can be subscribed multiple times to different events, but to only one timer.
    /// Note that multiple subscriptions to the same event will result in multiple events of the same type being sent.
    pub fn subscribe(
        &self,
        spell_id: SpellId,
        config: SpellTriggerConfigs,
    ) -> BoxFuture<'static, Result<(), EventBusError>> {
        self.send(spell_id, Action::Subscribe(config))
    }

    /// Unsubscribe a spell from all events.
    pub fn unsubscribe(&self, spell_id: SpellId) -> BoxFuture<'static, Result<(), EventBusError>> {
        self.send(spell_id, Action::Unsubscribe)
    }
}
