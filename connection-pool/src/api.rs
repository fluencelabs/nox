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
use crate::{ConnectionPoolBehaviour, ConnectionPoolT, Contact};

use fluence_libp2p::{
    generate_swarm_event_type,
    types::{Inlet, OneshotInlet, OneshotOutlet, Outlet},
};
use particle_protocol::Particle;

use futures::{
    channel::{mpsc::unbounded, oneshot},
    future::BoxFuture,
    stream::BoxStream,
    FutureExt, StreamExt,
};
use libp2p::swarm::NetworkBehaviourEventProcess;
use libp2p::PeerId;
use std::convert::identity;

enum Command {
    Connect {
        contact: Contact,
        out: OneshotOutlet<bool>,
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
    Send {
        to: Contact,
        particle: Particle,
        out: OneshotOutlet<bool>,
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
        let api = ConnectionPoolApi { outlet };
        let inlet = Self {
            inlet,
            connection_pool,
        };
        (api, inlet)
    }

    fn execute(&mut self, cmd: Command) {
        match cmd {
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
        cx: &mut std::task::Context,
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

#[derive(Clone)]
pub struct ConnectionPoolApi {
    outlet: Outlet<Command>,
}

impl ConnectionPoolApi {
    fn execute<R, F>(&self, cmd: F) -> BoxFuture<'static, R>
    where
        R: Default + Send + Sync + 'static,
        F: FnOnce(OneshotOutlet<R>) -> Command,
    {
        let (out, inlet) = oneshot::channel();
        if let Err(_) = self.outlet.unbounded_send(cmd(out)) {
            return futures::future::ready(R::default()).boxed();
        }
        inlet.map(|r| r.unwrap_or_default()).boxed()
    }
}

impl ConnectionPoolT for ConnectionPoolApi {
    fn connect(&self, contact: Contact) -> BoxFuture<'static, bool> {
        self.execute(|out| Command::Connect { contact, out })
    }

    fn disconnect(&self, contact: Contact) -> BoxFuture<'static, bool> {
        self.execute(|out| Command::Disconnect { contact, out })
    }

    fn is_connected(&self, peer_id: PeerId) -> BoxFuture<'static, bool> {
        self.execute(|out| Command::IsConnected { peer_id, out })
    }

    fn get_contact(&self, peer_id: PeerId) -> BoxFuture<'static, Option<Contact>> {
        self.execute(|out| Command::GetContact { peer_id, out })
    }

    fn send(&self, to: Contact, particle: Particle) -> BoxFuture<'static, bool> {
        self.execute(|out| Command::Send { to, particle, out })
    }

    fn count_connections(&self) -> BoxFuture<'static, usize> {
        self.execute(|out| Command::CountConnections { out })
    }

    fn lifecycle_events(&self) -> BoxStream<'static, LifecycleEvent> {
        let (out, inlet) = unbounded();
        let cmd = Command::LifecycleEvents { out };
        if let Err(_) = self.outlet.unbounded_send(cmd) {
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
