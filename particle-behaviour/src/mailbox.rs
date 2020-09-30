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

use futures::{channel::mpsc, StreamExt};
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::task::{Context, Poll};

type Closure = Arc<dyn Fn(Args) -> Option<IValue> + Send + Sync + 'static>;

type WaitResult = std_mpsc::Receiver<BuiltinCommandResult>;
type WaitingVM = std_mpsc::Sender<BuiltinCommandResult>;
type Inbox = mpsc::UnboundedReceiver<Command>;
type Destination = mpsc::UnboundedSender<Command>;

type Key = libp2p::kad::record::Key;
type Value = Vec<u8>;

#[derive(Debug)]
pub struct Command {
    outlet: WaitingVM,
    kind: BuiltinCommand,
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

#[derive(Debug, Clone)]
pub struct BuiltinServices {
    mailbox: Destination,
}

impl BuiltinServices {
    const SERVICES: &'static [&'static str] = &["services"];

    pub fn is_builtin(service_id: &str) -> bool {
        Self::SERVICES.contains(&service_id)
    }

    pub fn router(self) -> Closure {
        Arc::new(move |args| Some(Self::route(self.clone(), args).into()))
    }

    pub fn resolve(&self, key: Key) -> WaitResult {
        let (outlet, inlet) = std_mpsc::channel();
        let cmd = Command {
            outlet,
            kind: BuiltinCommand::DHTResolve(key),
        };
        self.mailbox
            .unbounded_send(cmd)
            .expect("builtin => mailbox");

        inlet
    }

    fn route(api: BuiltinServices, args: Args) -> BuiltinCommandResult {
        let wait = match args.service_id.as_str() {
            "resolve" => {
                let key = args
                    .args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .and_then(|s| bs58::decode(s).into_vec().ok())
                    .unwrap_or_else(|| unimplemented!("FIXME: return error?"));

                api.resolve(key.into())
            }
            "add_certificate" => unimplemented!("FIXME"),
            _ => unimplemented!("FIXME: unknown. return error? re-route to call service?"),
        };

        wait.recv().expect("receive BuiltinCommandResult")
    }
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
        BuiltinServices {
            mailbox: self.get_destination(),
        }
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
