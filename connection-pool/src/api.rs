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

use crate::connection_pool::LifecycleEvent;
use crate::{ConnectionPoolBehaviour, ConnectionPoolT};
use particle_protocol::Contact;

use fluence_libp2p::{
    generate_swarm_event_type,
    types::{Inlet, OneshotOutlet, Outlet},
};
use particle_protocol::Particle;

use futures::{
    channel::{mpsc::unbounded, oneshot},
    future::BoxFuture,
    stream::BoxStream,
    FutureExt, StreamExt,
};
use libp2p::{core::Multiaddr, swarm::NetworkBehaviourEventProcess, PeerId};
use std::time::Duration;

// marked `pub` to be available in benchmarks
#[derive(Debug)]
pub enum Command {
    Connect {
        contact: Contact,
        out: OneshotOutlet<bool>,
    },
    Send {
        to: Contact,
        particle: Particle,
        out: OneshotOutlet<bool>,
    },
    Dial {
        addr: Multiaddr,
        out: OneshotOutlet<Option<Contact>>,
    },
    Disconnect {
        contact: Contact,
        out: OneshotOutlet<bool>,
    },
    IsConnected {
        peer_id: PeerId,
        out: OneshotOutlet<bool>,
    },
    GetContact {
        peer_id: PeerId,
        out: OneshotOutlet<Option<Contact>>,
    },

    CountConnections {
        out: OneshotOutlet<usize>,
    },
    LifecycleEvents {
        out: Outlet<LifecycleEvent>,
    },
}

pub type SwarmEventType = generate_swarm_event_type!(ConnectionPoolInlet);

#[derive(::libp2p::NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll")]
pub struct ConnectionPoolInlet {
    #[behaviour(ignore)]
    inlet: Inlet<Command>,
    connection_pool: ConnectionPoolBehaviour,
}

impl ConnectionPoolInlet {
    pub fn new(connection_pool: ConnectionPoolBehaviour) -> (ConnectionPoolApi, Self) {
        let (outlet, inlet) = unbounded();
        let api = ConnectionPoolApi {
            outlet,
            send_timeout: connection_pool.protocol_config.upgrade_timeout,
        };
        let inlet = Self {
            inlet,
            connection_pool,
        };
        (api, inlet)
    }

    fn execute(&mut self, cmd: Command) {
        match cmd {
            Command::Dial { addr, out } => self.connection_pool.dial(addr, out),
            Command::Connect { contact, out } => self.connection_pool.connect(contact, out),
            Command::Disconnect { contact, out } => self.connection_pool.disconnect(contact, out),
            Command::IsConnected { peer_id, out } => {
                self.connection_pool.is_connected(peer_id, out)
            }
            Command::GetContact { peer_id, out } => self.connection_pool.get_contact(peer_id, out),
            Command::Send { to, particle, out } => self.connection_pool.send(to, particle, out),
            Command::CountConnections { out } => self.connection_pool.count_connections(out),
            Command::LifecycleEvents { out } => self.connection_pool.add_subscriber(out),
        }
    }

    fn custom_poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
        _: &mut impl libp2p::swarm::PollParameters,
    ) -> std::task::Poll<SwarmEventType> {
        use std::task::Poll;

        while let Poll::Ready(Some(cmd)) = self.inlet.poll_next_unpin(cx) {
            self.execute(cmd)
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<()> for ConnectionPoolInlet {
    fn inject_event(&mut self, _: ()) {}
}

#[derive(Clone, Debug)]
pub struct ConnectionPoolApi {
    // TODO: marked as `pub` to be available in benchmarks
    pub outlet: Outlet<Command>,
    pub send_timeout: Duration,
}

impl ConnectionPoolApi {
    fn execute<R, F>(&self, cmd: F) -> BoxFuture<'static, R>
    where
        R: Default + Send + Sync + 'static,
        F: FnOnce(OneshotOutlet<R>) -> Command,
    {
        let (out, inlet) = oneshot::channel();
        if self.outlet.unbounded_send(cmd(out)).is_err() {
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

    fn disconnect(&self, contact: Contact) -> BoxFuture<'static, bool> {
        // TODO: timeout needed? will be clearer when disconnect is implemented
        self.execute(|out| Command::Disconnect { contact, out })
    }

    fn is_connected(&self, peer_id: PeerId) -> BoxFuture<'static, bool> {
        // timeout isn't needed because result is returned immediately
        self.execute(|out| Command::IsConnected { peer_id, out })
    }

    fn get_contact(&self, peer_id: PeerId) -> BoxFuture<'static, Option<Contact>> {
        // timeout isn't needed because result is returned immediately
        self.execute(|out| Command::GetContact { peer_id, out })
    }

    fn send(&self, to: Contact, particle: Particle) -> BoxFuture<'static, bool> {
        let fut = self.execute(|out| Command::Send { to, particle, out });
        // timeout on send is required because libp2p can silently drop outbound events
        async_std::io::timeout(self.send_timeout, fut.map(Ok))
            // convert timeout to false
            .map(|r| r.unwrap_or(false))
            .boxed()
    }

    fn count_connections(&self) -> BoxFuture<'static, usize> {
        // timeout isn't needed because result is returned immediately
        self.execute(|out| Command::CountConnections { out })
    }

    fn lifecycle_events(&self) -> BoxStream<'static, LifecycleEvent> {
        let (out, inlet) = unbounded();
        let cmd = Command::LifecycleEvents { out };
        if self.outlet.unbounded_send(cmd).is_err() {
            return futures::stream::empty().boxed();
        };

        inlet.boxed()
    }
}

impl From<ConnectionPoolBehaviour> for (ConnectionPoolApi, ConnectionPoolInlet) {
    fn from(cpb: ConnectionPoolBehaviour) -> Self {
        ConnectionPoolInlet::new(cpb)
    }
}
