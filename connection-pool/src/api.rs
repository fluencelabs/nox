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

use std::time::Duration;

use futures::{future::BoxFuture, stream::BoxStream, FutureExt, StreamExt};
use libp2p::{core::Multiaddr, PeerId};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::UnboundedReceiverStream;

use particle_protocol::Particle;
use particle_protocol::{Contact, SendStatus};

use crate::connection_pool::LifecycleEvent;
use crate::ConnectionPoolT;

// marked `pub` to be available in benchmarks
#[derive(Debug)]
pub enum Command {
    Connect {
        contact: Contact,
        out: oneshot::Sender<bool>,
    },
    Send {
        to: Contact,
        particle: Particle,
        out: oneshot::Sender<SendStatus>,
    },
    Dial {
        addr: Multiaddr,
        out: oneshot::Sender<Option<Contact>>,
    },
    Disconnect {
        peer_id: PeerId,
        out: oneshot::Sender<bool>,
    },
    IsConnected {
        peer_id: PeerId,
        out: oneshot::Sender<bool>,
    },
    GetContact {
        peer_id: PeerId,
        out: oneshot::Sender<Option<Contact>>,
    },

    CountConnections {
        out: oneshot::Sender<usize>,
    },
    LifecycleEvents {
        out: mpsc::UnboundedSender<LifecycleEvent>,
    },
}

#[derive(Clone, Debug)]
pub struct ConnectionPoolApi {
    // TODO: marked as `pub` to be available in benchmarks
    pub outlet: UnboundedSender<Command>,
    pub send_timeout: Duration,
}

impl ConnectionPoolApi {
    fn execute<R, F>(&self, cmd: F) -> BoxFuture<'static, R>
    where
        R: Default + Send + Sync + 'static,
        F: FnOnce(oneshot::Sender<R>) -> Command,
    {
        let (out, inlet) = oneshot::channel();
        if self.outlet.send(cmd(out)).is_err() {
            return futures::future::ready(R::default()).boxed();
        }
        inlet.map(|r| r.unwrap_or_default()).boxed()
    }
}

impl ConnectionPoolT for ConnectionPoolApi {
    fn dial(&self, addr: Multiaddr) -> BoxFuture<'static, Option<Contact>> {
        // timeout isn't needed because libp2p handles it through inject_dial_failure, etc
        self.execute(|out| Command::Dial { addr, out })
    }

    fn connect(&self, contact: Contact) -> BoxFuture<'static, bool> {
        // timeout isn't needed because libp2p handles it through inject_dial_failure, etc
        self.execute(|out| Command::Connect { contact, out })
    }

    fn disconnect(&self, peer_id: PeerId) -> BoxFuture<'static, bool> {
        // TODO: timeout needed? will be clearer when disconnect is implemented
        self.execute(|out| Command::Disconnect { peer_id, out })
    }

    fn is_connected(&self, peer_id: PeerId) -> BoxFuture<'static, bool> {
        // timeout isn't needed because result is returned immediately
        self.execute(|out| Command::IsConnected { peer_id, out })
    }

    fn get_contact(&self, peer_id: PeerId) -> BoxFuture<'static, Option<Contact>> {
        // timeout isn't needed because result is returned immediately
        self.execute(|out| Command::GetContact { peer_id, out })
    }

    fn send(&self, to: Contact, particle: Particle) -> BoxFuture<'static, SendStatus> {
        let fut = self.execute(|out| Command::Send { to, particle, out });
        // timeout on send is required because libp2p can silently drop outbound events
        let timeout = self.send_timeout;
        tokio::time::timeout(self.send_timeout, fut)
            // convert timeout to false
            .map(move |r| match r {
                Ok(status) => status,
                Err(error) => {
                    let error = error.into();
                    SendStatus::TimedOut {
                        after: timeout,
                        error,
                    }
                }
            })
            .boxed()
    }

    fn count_connections(&self) -> BoxFuture<'static, usize> {
        // timeout isn't needed because result is returned immediately
        self.execute(|out| Command::CountConnections { out })
    }

    fn lifecycle_events(&self) -> BoxStream<'static, LifecycleEvent> {
        let (out, inlet) = unbounded_channel();
        let cmd = Command::LifecycleEvents { out };
        if self.outlet.send(cmd).is_err() {
            return futures::stream::empty().boxed();
        };

        UnboundedReceiverStream::new(inlet).boxed()
    }
}
