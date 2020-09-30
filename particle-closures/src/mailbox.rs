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

use particle_services::{Args, IValue};
use waiting_queues::WaitingQueues;

use crate::builtin_services::BuiltinServices;
use futures::{channel::mpsc, StreamExt};
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::task::{Context, Poll};

pub(super) type Closure = Arc<dyn Fn(Args) -> Option<IValue> + Send + Sync + 'static>;

pub(super) type WaitResult = std_mpsc::Receiver<BuiltinCommandResult>;
pub(super) type WaitingVM = std_mpsc::Sender<BuiltinCommandResult>;
pub(super) type Inbox = mpsc::UnboundedReceiver<Command>;
pub(super) type Destination = mpsc::UnboundedSender<Command>;

pub(super) type Key = libp2p::kad::record::Key;
pub(super) type Value = Vec<u8>;

#[derive(Debug)]
pub struct Command {
    pub outlet: WaitingVM,
    pub kind: BuiltinCommand,
}

#[derive(Debug, Clone)]
pub enum BuiltinCommand {
    DHTResolve(Key),
}

impl BuiltinCommand {
    pub fn key(&self) -> &Key {
        match self {
            BuiltinCommand::DHTResolve(key) => &key,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BuiltinCommandResult {
    DHTResolved(Key, Value),
}

impl Into<IValue> for BuiltinCommandResult {
    fn into(self) -> IValue {
        unimplemented!("FIXME")
    }
}

#[derive(Debug)]
pub struct Mailbox {
    waiting: WaitingQueues<Key, Command>,
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

    pub fn get_api(&self) -> BuiltinServices {
        BuiltinServices::new(self.get_destination())
    }
}

// Behaviour API
impl Mailbox {
    pub fn resolve_complete(&mut self, key: Key, value: Value) {
        for cmd in self.waiting.remove(&key) {
            let result = BuiltinCommandResult::DHTResolved(key.clone(), value.clone());
            cmd.outlet.send(result.clone()).expect("resolve_complete")
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<BuiltinCommand> {
        match self.inbox.poll_next_unpin(cx) {
            Poll::Ready(Some(cmd)) => {
                let kind = cmd.kind.clone();
                self.waiting.enqueue(kind.key().clone(), cmd);
                Poll::Ready(kind)
            }
            Poll::Ready(None) => unreachable!("destination couldn't be dropped"),
            Poll::Pending => Poll::Pending,
        }
    }
}
