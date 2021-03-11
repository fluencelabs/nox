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

pub use fluence_client::ClientEvent;

use super::misc::Result;
use crate::{
    make_call_service_closure, make_particle, make_swarms, make_vm, read_args, timeout,
    CreatedSwarm, KAD_TIMEOUT, SHORT_TIMEOUT, TIMEOUT,
};

use aquamarine_vm::AquamarineVM;
use fluence_client::{Client, Transport};
use particle_protocol::Particle;

use async_std::task;
use core::ops::Deref;
use eyre::{bail, WrapErr};
use libp2p::{core::Multiaddr, identity::ed25519, PeerId};
use serde_json::Value as JValue;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::Arc;

use parking_lot::Mutex;
use std::time::Duration;

pub struct ConnectedClient {
    pub client: Client,
    pub node: PeerId,
    pub node_address: Multiaddr,
    pub timeout: Duration,
    pub short_timeout: Duration,
    pub kad_timeout: Duration,
    pub call_service_in: Arc<Mutex<HashMap<String, JValue>>>,
    pub call_service_out: Arc<Mutex<Vec<JValue>>>,
    pub local_vm: AquamarineVM,
}

impl ConnectedClient {
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn short_timeout(&self) -> Duration {
        self.short_timeout
    }

    pub fn kad_timeout(&self) -> Duration {
        self.kad_timeout
    }
}

impl Deref for ConnectedClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for ConnectedClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

impl ConnectedClient {
    pub fn connect_to(node_address: Multiaddr) -> Result<Self> {
        Self::connect_to_with_peer_id(node_address, None)
    }

    pub fn connect_to_with_peer_id(
        node_address: Multiaddr,
        key_pair: Option<ed25519::Keypair>,
    ) -> Result<Self> {
        use core::result::Result;
        use std::io::{Error, ErrorKind};

        let transport = Transport::from_maddr(&node_address);
        let connect = async move {
            let (mut client, _) = Client::connect_with(node_address.clone(), transport, key_pair)
                .await
                .expect("sender connected");
            let result: Result<_, Error> = if let Some(ClientEvent::NewConnection {
                peer_id, ..
            }) = client.receive_one().await
            {
                Ok(ConnectedClient::new(client, peer_id, node_address))
            } else {
                Err(ErrorKind::ConnectionAborted.into())
            };

            result
        };
        Ok(task::block_on(timeout(TIMEOUT, connect))??)
    }

    pub fn new(client: Client, node: PeerId, node_address: Multiaddr) -> Self {
        let call_service_in: Arc<Mutex<HashMap<String, JValue>>> = <_>::default();
        let call_service_out: Arc<Mutex<Vec<JValue>>> = <_>::default();
        let local_vm = make_vm(
            &client.peer_id,
            make_call_service_closure(call_service_in.clone(), call_service_out.clone()),
        );

        Self {
            client,
            node,
            node_address,
            timeout: TIMEOUT,
            short_timeout: SHORT_TIMEOUT,
            kad_timeout: KAD_TIMEOUT,
            call_service_in,
            call_service_out,
            local_vm,
        }
    }

    pub fn make_clients() -> Result<(Self, Self)> {
        let swarms = make_swarms(3);
        let mut swarms = swarms.into_iter();
        let CreatedSwarm(peer_id1, addr1, ..) = swarms.next().expect("get swarm");
        let CreatedSwarm(peer_id2, addr2, ..) = swarms.next().expect("get swarm");

        let connect = async move {
            let (mut first, _) = Client::connect_with(addr1.clone(), Transport::Memory, None)
                .await
                .expect("first connected");
            first.receive_one().await;

            let first = ConnectedClient::new(first, peer_id1, addr1);

            let (mut second, _) = Client::connect_with(addr2.clone(), Transport::Memory, None)
                .await
                .expect("second connected");
            second.receive_one().await;

            let second = ConnectedClient::new(second, peer_id2, addr2);

            (first, second)
        };

        task::block_on(timeout(TIMEOUT, connect))
    }

    pub fn send(&self, particle: Particle) {
        self.client.send(particle, self.node)
    }

    pub fn send_particle(
        &mut self,
        script: impl Into<String>,
        data: HashMap<&'static str, JValue>,
    ) -> String {
        *self.call_service_in.lock() = data
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect();
        let particle = make_particle(
            self.peer_id,
            self.call_service_in.clone(),
            script.into(),
            self.node,
            &mut self.local_vm,
        );
        let id = particle.id.clone();
        self.send(particle);
        id
    }

    pub fn maybe_receive(&mut self) -> Option<Particle> {
        let short_timeout = self.short_timeout();
        let receive = self.client.receive_one();
        let particle = task::block_on(timeout(short_timeout, receive)).ok()??;

        match particle {
            ClientEvent::Particle { particle, .. } => Some(particle),
            _ => None,
        }
    }

    pub fn receive(&mut self) -> Result<Particle> {
        let tout = self.timeout();
        let receive = self.client.receive_one();
        let result = task::block_on(timeout(tout, receive)).wrap_err("receive particle")?;

        if let Some(ClientEvent::Particle { particle, .. }) = result {
            Ok(particle)
        } else {
            bail!("Expected Some(Particle), got {:?}", result)
        }
    }

    pub fn receive_args(&mut self) -> Result<Vec<JValue>> {
        let particle = self.receive().wrap_err("receive_args")?;
        Ok(read_args(
            particle,
            self.peer_id.clone(),
            &mut self.local_vm,
            self.call_service_out.clone(),
        ))
    }

    /// Wait for a particle with specified `particle_id`, and read "op" "return" result from it
    pub fn wait_particle_args(&mut self, particle_id: String) -> Result<Vec<JValue>> {
        let mut max = 100;
        loop {
            max -= 1;
            if max <= 0 {
                bail!("timed out waiting for particle {}", particle_id);
            }
            let particle = self.receive().ok();
            if let Some(particle) = particle {
                if particle.id == particle_id {
                    break Ok(read_args(
                        particle,
                        self.peer_id.clone(),
                        &mut self.local_vm,
                        self.call_service_out.clone(),
                    ));
                }
            }
        }
    }
}
