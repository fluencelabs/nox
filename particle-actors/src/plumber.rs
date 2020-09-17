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

use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use particle_protocol::Particle;
use std::collections::{HashMap, VecDeque};
use std::task::{Poll, Waker};

#[derive(Debug)]
pub enum PeerKind {
    Client,
    Unknown,
}

#[derive(Debug)]
pub enum PlumberEvent {
    Forward {
        target: PeerId,
        particle: Particle,
        kind: PeerKind,
    },
}

pub struct Plumber {
    clients: HashMap<PeerId, Option<Multiaddr>>,
    events: VecDeque<PlumberEvent>,
    pub(super) waker: Option<Waker>,
}

impl Plumber {
    pub fn new() -> Self {
        Self {
            clients: <_>::default(),
            events: <_>::default(),
            waker: <_>::default(),
        }
    }

    pub fn add_client(&mut self, client: PeerId) {
        self.clients.insert(client, None);
    }

    pub fn add_client_address(&mut self, client: PeerId, address: Multiaddr) {
        self.clients.insert(client, Some(address));
    }

    pub fn client_address(&self, client: &PeerId) -> &Option<Multiaddr> {
        self.clients.get(client).unwrap_or(&None)
    }

    pub fn remove_client(&mut self, client: &PeerId) {
        self.clients.remove(client);
    }

    pub fn ingest(&mut self, particle: Particle) {
        let target = particle.init_peer_id.clone();

        let kind = if self.clients.contains_key(&target) {
            PeerKind::Client
        } else {
            PeerKind::Unknown
        };

        self.emit(PlumberEvent::Forward {
            target,
            kind,
            particle,
        });
    }

    pub fn poll(&mut self, waker: Waker) -> Poll<PlumberEvent> {
        self.waker = Some(waker);

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }

    fn emit(&mut self, event: PlumberEvent) {
        if let Some(waker) = self.waker.clone() {
            waker.wake();
        }

        self.events.push_back(event);
    }
}
