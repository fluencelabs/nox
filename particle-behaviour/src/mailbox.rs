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

type WaitResult = std_mpsc::Receiver<DHTResult>;
type WaitingVM = std_mpsc::Sender<DHTResult>;
type Inbox = mpsc::UnboundedReceiver<DHTCommand>;
type Destination = mpsc::UnboundedSender<DHTCommand>;

type Key = libp2p::kad::record::Key;
type Value = Vec<u8>;

#[derive(Debug)]
pub struct DHTCommand {
    outlet: WaitingVM,
    kind: DHTCommandKind,
}

#[derive(Debug, Clone)]
pub enum DHTCommandKind {
    Resolve(Key),
}

impl DHTCommandKind {
    pub fn key(&self) -> &Key {
        match self {
            DHTCommandKind::Resolve(key) => &key,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DHTResult {
    Resolved(Key, Value),
}

impl Into<IValue> for DHTResult {
    fn into(self) -> IValue {
        unimplemented!("FIXME")
    }
}

#[derive(Debug)]
pub struct Mailbox {
    waiting: WaitingQueues<Key, DHTCommand>,
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

    fn route(api: BuiltinServices, args: Args) -> DHTResult {
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
            _ => unimplemented!("FIXME: unknown. return error? re-route to call service?"),
        };

        wait.recv().expect("receive DHTResult")
    }

    pub fn resolve(&self, key: Key) -> WaitResult {
        let (outlet, inlet) = std_mpsc::channel();
        let cmd = DHTCommand {
            outlet,
            kind: DHTCommandKind::Resolve(key),
        };
        self.mailbox.unbounded_send(cmd).expect("api => mailbox");

        inlet
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
    pub fn get_destination(&self) -> Destination {
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
            let result = DHTResult::Resolved(key.clone(), value.clone());
            cmd.outlet.send(result.clone()).expect("resolve_complete")
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<DHTCommandKind> {
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
