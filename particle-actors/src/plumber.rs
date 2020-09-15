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
use std::collections::{HashMap, HashSet};

pub struct Plumber {
    clients: HashMap<PeerId, Option<Multiaddr>>,
}

impl Plumber {
    pub fn new() -> Self {
        Self {
            clients: <_>::default(),
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
        unimplemented!()
    }
}
