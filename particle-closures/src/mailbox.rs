/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::builtin_services_api::BuiltinServicesApi;

use json_utils::err_as_value;
use particle_dht::{NeighborhoodError, ResolveErrorKind};
use waiting_queues::WaitingQueues;

use futures::{channel::mpsc, StreamExt};
use libp2p::{kad::record, PeerId};
use serde_json::{json, Value as JValue};
use std::{
    collections::HashSet,
    sync::mpsc as std_mpsc,
    task::{Context, Poll},
};

pub(super) type WaitResult = std_mpsc::Receiver<BuiltinCommandResult>;
pub(super) type WaitingVM = std_mpsc::Sender<BuiltinCommandResult>;
pub(super) type Inbox = mpsc::UnboundedReceiver<Command>;
pub(super) type Destination = mpsc::UnboundedSender<Command>;

#[derive(Debug)]
pub struct Command {
    pub outlet: WaitingVM,
    pub kind: BuiltinCommand,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum BuiltinCommand {
    DHTResolve(record::Key),
    DHTNeighborhood(Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum BuiltinCommandResult {
    DHTResolved(Result<Vec<Vec<u8>>, ResolveErrorKind>),
    DHTNeighborhood(Result<Vec<PeerId>, NeighborhoodError>),
}

impl Into<Result<JValue, JValue>> for BuiltinCommandResult {
    fn into(self) -> Result<JValue, JValue> {
        match self {
            BuiltinCommandResult::DHTResolved(v) => v.map(|vs| json!(vs)).map_err(err_as_value),
            BuiltinCommandResult::DHTNeighborhood(v) => v
                .map(|vs| json!(vs.into_iter().map(|id| id.to_string()).collect::<Vec<_>>()))
                .map_err(err_as_value),
        }
    }
}

#[derive(Debug)]
pub struct Mailbox {
    waiting: WaitingQueues<BuiltinCommand, WaitingVM>,
    inbox: Inbox,
    destination: Destination,
}

// Infrastructure
impl Mailbox {
    pub fn new() -> Self {
        let (destination, inbox) = mpsc::unbounded();
        Self {
            inbox,
            destination,
            waiting: <_>::default(),
        }
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

// VM API
impl Mailbox {
    fn get_destination(&self) -> Destination {
        self.destination.clone()
    }

    pub fn get_api(&self) -> BuiltinServicesApi {
        BuiltinServicesApi::new(self.get_destination())
    }
}

/// Behaviour API
impl Mailbox {
    /// Called when `dht.get_record` for `key` is completed
    pub fn resolve_complete(
        &mut self,
        key: record::Key,
        value: Result<HashSet<Vec<u8>>, ResolveErrorKind>,
    ) {
        let value = value.map(|v| v.into_iter().collect());
        let result = BuiltinCommandResult::DHTResolved(value);
        for outlet in self.waiting.remove(&BuiltinCommand::DHTResolve(key)) {
            outlet.send(result.clone()).expect("resolve_complete")
        }
    }

    /// Called when `dht.get_closest_peers` is completed
    pub fn got_neighborhood(
        &mut self,
        key: Vec<u8>,
        value: Result<HashSet<PeerId>, NeighborhoodError>,
    ) {
        let value = value.map(|v| v.into_iter().collect());
        let result = BuiltinCommandResult::DHTNeighborhood(value);
        for outlet in self.waiting.remove(&BuiltinCommand::DHTNeighborhood(key)) {
            outlet.send(result.clone()).expect("resolve_complete")
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<BuiltinCommand> {
        match self.inbox.poll_next_unpin(cx) {
            Poll::Ready(Some(Command { kind, outlet })) => {
                // Enqueue VM's outlet to wait for command result
                self.waiting.enqueue(kind.clone(), outlet);
                // Return BuiltinCommand signaling outer world to start executing the command
                Poll::Ready(kind)
            }
            Poll::Ready(None) => unreachable!("destination couldn't be dropped"),
            Poll::Pending => Poll::Pending,
        }
    }
}
