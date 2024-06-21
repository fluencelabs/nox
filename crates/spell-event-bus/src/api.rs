/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use connection_pool::LifecycleEvent;
use fluence_libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use types::peer_id;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimerEvent {
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Event is triggered by connection pool event
pub struct PeerEvent {
    #[serde(
        serialize_with = "peer_id::serde::serialize",
        deserialize_with = "peer_id::serde::deserialize"
    )]
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

#[derive(Debug, Serialize, Deserialize)]
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

impl From<TriggerInfoAqua> for TriggerInfo {
    fn from(i: TriggerInfoAqua) -> Self {
        match (i.timer.first(), i.peer.first()) {
            (Some(t), None) => Self::Timer(t.clone()),
            (None, Some(p)) => Self::Peer(p.clone()),
            _ => unreachable!("TriggerInfoAqua should always have either timer or peer event"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Command {
    pub(crate) action: Action,
    pub(crate) reply: oneshot::Sender<()>,
}

#[derive(Debug, Clone)]
pub enum Action {
    /// Subscribe a spell to a list of triggers
    Subscribe(SpellId, SpellTriggerConfigs),
    /// Remove all subscriptions of a spell
    Unsubscribe(SpellId),
    /// Actually start the scheduling
    Start,
}

#[derive(Error, Debug)]
pub enum EventBusError {
    #[error("can't send a command `{action:?}` to spell-event-bus: {reason}")]
    SendError {
        action: Action,
        reason: Pin<Box<dyn std::error::Error + Send>>,
    },
    #[error("can't receive a message from the bus on behalf of a command {0:?}: sending end is probably dropped")]
    ReplyError(Action),
}

#[derive(Clone)]
pub struct SpellEventBusApi {
    pub(crate) send_cmd_channel: mpsc::UnboundedSender<Command>,
}

impl std::fmt::Debug for SpellEventBusApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpellEventBusApi").finish()
    }
}

impl SpellEventBusApi {
    async fn send(&self, action: Action) -> Result<(), EventBusError> {
        let (send, recv) = oneshot::channel();
        let command = Command {
            action: action.clone(),
            reply: send,
        };
        self.send_cmd_channel
            .send(command)
            .map_err(|e| EventBusError::SendError {
                action: action.clone(),
                reason: Box::pin(e),
            })?;

        recv.await.map_err(|_| EventBusError::ReplyError(action))?;
        Ok(())
    }

    /// Subscribe a spell to a list of events
    /// The spell can be subscribed multiple times to different events, but to only one timer.
    /// Note that multiple subscriptions to the same event will result in multiple events of the same type being sent.
    pub async fn subscribe(
        &self,
        spell_id: SpellId,
        config: SpellTriggerConfigs,
    ) -> Result<(), EventBusError> {
        self.send(Action::Subscribe(spell_id, config)).await
    }

    /// Unsubscribe a spell from all events.
    pub async fn unsubscribe(&self, spell_id: SpellId) -> Result<(), EventBusError> {
        self.send(Action::Unsubscribe(spell_id)).await
    }

    pub async fn start_scheduling(&self) -> Result<(), EventBusError> {
        self.send(Action::Start).await
    }
}
