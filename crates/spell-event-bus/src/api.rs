use crate::config::SpellTriggerConfigs;
use connection_pool::LifecycleEvent;
use fluence_libp2p::types::{OneshotOutlet, Outlet};
use fluence_libp2p::{peerid_serializer, PeerId};
use futures::channel::mpsc::SendError;
use futures::{channel::oneshot, future::BoxFuture, FutureExt};
use serde::Serialize;
use thiserror::Error;

pub use crate::config::*;

pub type SpellId = String;

#[derive(Debug, Clone)]
pub struct TriggerEvent {
    pub spell_id: SpellId,
    pub info: TriggerInfo,
}

#[derive(Clone, Debug)]
pub enum TriggerInfo {
    /// Event is triggered by timer.
    Timer(TimerEvent),
    /// Event is triggered by a peer event.
    Peer(PeerEvent),
}

#[derive(Clone, Debug, Serialize)]
pub struct TimerEvent {
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize)]
/// Event is triggered by connection pool event
pub struct PeerEvent {
    #[serde(with = "peerid_serializer")]
    pub peer_id: PeerId,
    pub connected: bool,
}

impl From<LifecycleEvent> for PeerEvent {
    fn from(e: LifecycleEvent) -> Self {
        match e {
            LifecycleEvent::Connected(c) => Self {
                peer_id: c.peer_id,
                connected: true,
            },
            LifecycleEvent::Disconnected(c) => Self {
                peer_id: c.peer_id,
                connected: false,
            },
        }
    }
}

impl PeerEvent {
    pub(crate) fn get_type(&self) -> PeerEventType {
        if self.connected {
            PeerEventType::Connected
        } else {
            PeerEventType::Disconnected
        }
    }
}

/// Types of events that are available for subscription.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum PeerEventType {
    Connected,
    Disconnected,
}

#[derive(Serialize)]
pub struct TriggerInfoAqua {
    // Vec is a representation for Aqua optional values. This Vec always holds at most 1 element.
    timer: Vec<TimerEvent>,
    // Vec is a representation for Aqua optional values. This Vec always holds at most 1 element.
    peer: Vec<PeerEvent>,
}

impl From<TriggerInfo> for TriggerInfoAqua {
    fn from(i: TriggerInfo) -> Self {
        match i {
            TriggerInfo::Timer(t) => Self {
                timer: vec![t],
                peer: vec![], // Empty Vec corresponds to Aqua nil
            },
            TriggerInfo::Peer(p) => Self {
                timer: vec![], // Empty Vec corresponds to Aqua nil
                peer: vec![p],
            },
        }
    }
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
    /// Update trigger config of an existing spell
    Update(SpellTriggerConfigs),
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

    pub fn update_config(
        &self,
        spell_id: SpellId,
        config: SpellTriggerConfigs,
    ) -> BoxFuture<'static, Result<(), EventBusError>> {
        self.send(spell_id, Action::Update(config))
    }
}
