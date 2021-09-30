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

use core::ops::Deref;
use std::{collections::HashMap, lazy::Lazy, ops::DerefMut, sync::Arc, time::Duration};

use async_std::task;
use eyre::Result;
use eyre::{bail, WrapErr};
use fluence_identity::KeyPair;
use libp2p::{core::Multiaddr, identity::Keypair, PeerId};
use parking_lot::Mutex;
use serde_json::Value as JValue;

use fluence_libp2p::Transport;
use local_vm::{make_call_service_closure, make_particle, make_vm, read_args, DataStoreError};
use particle_protocol::Particle;
use test_constants::{KAD_TIMEOUT, PARTICLE_TTL, SHORT_TIMEOUT, TIMEOUT, TRANSPORT_TIMEOUT};

use crate::client::Client;
use crate::event::ClientEvent;

type AVM = local_vm::AVM<DataStoreError>;

pub struct ConnectedClient {
    pub client: Client,
    pub node: PeerId,
    pub node_address: Multiaddr,
    pub timeout: Duration,
    pub short_timeout: Duration,
    pub kad_timeout: Duration,
    pub call_service_in: Arc<Mutex<HashMap<String, JValue>>>,
    pub call_service_out: Arc<Mutex<Vec<JValue>>>,
    pub local_vm: Lazy<Mutex<AVM>, Box<dyn FnOnce() -> Mutex<AVM>>>,
    pub particle_ttl: Duration,
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

    pub fn particle_ttl(&self) -> Duration {
        self.particle_ttl
    }

    pub fn set_particle_ttl(&mut self, particle_ttl: Duration) {
        self.particle_ttl = particle_ttl;
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
        Self::connect_with_keypair(node_address, None)
    }

    pub fn connect_to_with_timeout(
        node_address: Multiaddr,
        timeout: Duration,
        particle_ttl: Option<Duration>,
    ) -> Result<Self> {
        Self::connect_with_timeout(node_address, None, timeout, particle_ttl)
    }

    pub fn connect_with_keypair(
        node_address: Multiaddr,
        key_pair: Option<KeyPair>,
    ) -> Result<Self> {
        Self::connect_with_timeout(node_address, key_pair, TRANSPORT_TIMEOUT, None)
    }

    pub fn connect_with_timeout(
        node_address: Multiaddr,
        key_pair: Option<KeyPair>,
        timeout: Duration,
        particle_ttl: Option<Duration>,
    ) -> Result<Self> {
        use core::result::Result;
        use std::io::{Error, ErrorKind};

        let transport = Transport::from_maddr(&node_address);
        let connect = async move {
            let (mut client, _) = Client::connect_with(
                node_address.clone(),
                transport,
                key_pair.map(Into::into),
                timeout,
            )
            .await
            .expect("sender connected");
            let result: Result<_, Error> = if let Some(ClientEvent::NewConnection {
                peer_id, ..
            }) = client.receive_one().await
            {
                Ok(ConnectedClient::new(
                    client,
                    peer_id,
                    node_address,
                    particle_ttl,
                ))
            } else {
                Err(ErrorKind::ConnectionAborted.into())
            };

            result
        };
        Ok(task::block_on(self::timeout(TIMEOUT, connect))??)
    }

    pub fn new(
        client: Client,
        node: PeerId,
        node_address: Multiaddr,
        particle_ttl: Option<Duration>,
    ) -> Self {
        let call_service_in: Arc<Mutex<HashMap<String, JValue>>> = <_>::default();
        let call_service_out: Arc<Mutex<Vec<JValue>>> = <_>::default();

        let peer_id = client.peer_id;
        let call_in = call_service_in.clone();
        let call_out = call_service_out.clone();
        let f: Box<dyn FnOnce() -> Mutex<AVM>> = Box::new(move || {
            Mutex::new(make_vm(
                peer_id, /*make_call_service_closure(call_in, call_out)*/
            ))
        });
        let local_vm = Lazy::new(f);

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
            particle_ttl: particle_ttl.unwrap_or(Duration::from_millis(PARTICLE_TTL as u64)),
        }
    }

    pub fn send(&self, particle: Particle) {
        self.client.send(particle, self.node)
    }

    pub fn send_particle(
        &mut self,
        script: impl Into<String>,
        data: HashMap<&str, JValue>,
    ) -> String {
        self.send_particle_ext(script, data, false)
    }

    pub fn send_particle_ext(
        &mut self,
        script: impl Into<String>,
        data: HashMap<&str, JValue>,
        generated: bool,
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
            &mut self.local_vm.lock(),
            generated,
            self.particle_ttl(),
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
        let result = task::block_on(timeout(tout, async {
            loop {
                let result = self.client.receive_one().await;
                if let Some(ClientEvent::Particle { particle, .. }) = result {
                    break particle;
                }
            }
        }))
        .wrap_err("receive particle")?;

        Ok(result)
    }

    pub fn receive_args(&mut self) -> Result<Vec<JValue>> {
        let particle = self.receive().wrap_err("receive_args")?;
        Ok(read_args(
            particle,
            self.peer_id,
            &mut self.local_vm.lock(),
            self.call_service_out.clone(),
        ))
    }

    /// Wait for a particle with specified `particle_id`, and read "op" "return" result from it
    pub fn wait_particle_args(&mut self, particle_id: impl AsRef<str>) -> Result<Vec<JValue>> {
        let mut max = 100;
        loop {
            max -= 1;
            if max <= 0 {
                bail!("timed out waiting for particle {}", particle_id.as_ref());
            }
            let particle = self.receive().ok();
            if let Some(particle) = particle {
                if &particle.id == particle_id.as_ref() {
                    break Ok(read_args(
                        particle,
                        self.peer_id,
                        &mut self.local_vm.lock(),
                        self.call_service_out.clone(),
                    ));
                }
            }
        }
    }

    pub fn listen_for_n<F: Fn(Vec<JValue>)>(&mut self, mut n: usize, f: F) {
        loop {
            n -= 1;
            if n <= 0 {
                break;
            }

            let particle = self.receive().ok();
            if let Some(particle) = particle {
                println!("received particle {}", particle.id);
                let args = read_args(
                    particle,
                    self.peer_id,
                    &mut self.local_vm.lock(),
                    self.call_service_out.clone(),
                );
                f(args);
            }
        }
    }
}

pub async fn timeout<F, T>(dur: Duration, f: F) -> eyre::Result<T>
where
    F: std::future::Future<Output = T>,
{
    Ok(async_std::future::timeout(dur, f)
        .await
        .wrap_err(format!("timed out after {:?}", dur))?)
}
