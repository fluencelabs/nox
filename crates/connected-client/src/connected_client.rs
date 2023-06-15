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
use std::{cell::LazyCell, collections::HashMap, ops::DerefMut, time::Duration};

use eyre::Result;
use eyre::{bail, eyre, WrapErr};
use fluence_keypair::KeyPair;
use fluence_libp2p::Transport;
use libp2p::{core::Multiaddr, PeerId};
use local_vm::{make_particle, make_vm, read_args, DataStoreError};
use parking_lot::Mutex;
use particle_protocol::Particle;
use serde_json::{Value as JValue, Value};
use test_constants::{KAD_TIMEOUT, PARTICLE_TTL, SHORT_TIMEOUT, TIMEOUT, TRANSPORT_TIMEOUT};

use crate::client::Client;
use crate::event::ClientEvent;

#[allow(clippy::upper_case_acronyms)]
type AVM = local_vm::AVM<DataStoreError>;

pub struct ConnectedClient {
    pub client: Client,
    pub node: PeerId,
    pub node_address: Multiaddr,
    pub timeout: Duration,
    pub short_timeout: Duration,
    pub kad_timeout: Duration,
    pub local_vm: LazyCell<Mutex<AVM>, Box<dyn FnOnce() -> Mutex<AVM>>>,
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
    pub async fn connect_to(node_address: Multiaddr) -> Result<Self> {
        Self::connect_with_keypair(node_address, None).await
    }

    pub async fn connect_to_with_timeout(
        node_address: Multiaddr,
        timeout: Duration,
        particle_ttl: Option<Duration>,
    ) -> Result<Self> {
        Self::connect_with_timeout(node_address, None, timeout, particle_ttl).await
    }

    pub async fn connect_with_keypair(
        node_address: Multiaddr,
        key_pair: Option<KeyPair>,
    ) -> Result<Self> {
        Self::connect_with_timeout(node_address, key_pair, TRANSPORT_TIMEOUT, None).await
    }

    pub async fn connect_with_timeout(
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

        let result = self::timeout(TIMEOUT, connect).await??;

        Ok(result)
    }

    pub fn new(
        client: Client,
        node: PeerId,
        node_address: Multiaddr,
        particle_ttl: Option<Duration>,
    ) -> Self {
        let peer_id = client.peer_id;
        let f: Box<dyn FnOnce() -> Mutex<AVM>> = Box::new(move || Mutex::new(make_vm(peer_id)));
        let local_vm = LazyCell::new(f);

        Self {
            client,
            node,
            node_address,
            timeout: TIMEOUT,
            short_timeout: SHORT_TIMEOUT,
            kad_timeout: KAD_TIMEOUT,
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

    pub async fn execute_particle(
        &mut self,
        script: impl Into<String>,
        data: HashMap<&str, JValue>,
    ) -> Result<Vec<JValue>> {
        let particle_id = self.send_particle_ext(script, data, false);
        self.wait_particle_args(particle_id.clone()).await
    }

    pub fn send_particle_ext(
        &mut self,
        script: impl Into<String>,
        data: HashMap<&str, JValue>,
        generated: bool,
    ) -> String {
        let data = data
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect();
        let particle = make_particle(
            self.peer_id,
            &data,
            script.into(),
            self.node,
            &mut self.local_vm.lock(),
            generated,
            self.particle_ttl(),
            &self.key_pair,
        );
        let id = particle.id.clone();
        self.send(particle);
        id
    }

    pub async fn maybe_receive(&mut self) -> Option<Particle> {
        let short_timeout = self.short_timeout();
        let receive = self.client.receive_one();
        let particle = timeout(short_timeout, receive).await.ok()??;
        match particle {
            ClientEvent::Particle { particle, .. } => Some(particle),
            _ => None,
        }
    }

    pub async fn receive(&mut self) -> Result<Particle> {
        let head = self.fetched.pop();

        match head {
            Some(particle) => Ok(particle),
            None => self.raw_receive().await,
        }
    }

    async fn raw_receive(&mut self) -> Result<Particle> {
        let tout = self.timeout();
        let result = timeout(tout, async {
            loop {
                let result = self.client.receive_one().await;
                if let Some(ClientEvent::Particle { particle, .. }) = result {
                    break particle;
                }
            }
        })
        .await;
        let result = result.wrap_err("receive particle")?;

        Ok(result)
    }

    pub async fn receive_args(&mut self) -> Result<Vec<JValue>> {
        let particle = self.receive().await.wrap_err("receive_args")?;
        println!("{} received particle {}", self.peer_id, particle.id);
        let result = read_args(
            particle,
            self.peer_id,
            &mut self.local_vm.lock(),
            &self.key_pair,
        );
        match result {
            Some(result) => result.map_err(|args| eyre!("AIR caught an error: {:?}", args)),
            None => Err(eyre!("Received a particle, but it didn't return anything")),
        }
    }

    /// Wait for a particle with specified `particle_id`, and read "op" "return" result from it
    pub async fn wait_particle_args(
        &mut self,
        particle_id: impl AsRef<str>,
    ) -> Result<Vec<JValue>> {
        log::info!("wait_particle_args {:?}", self.fetched);
        let head = self
            .fetched
            .iter()
            .position(|particle| particle.id == particle_id.as_ref());

        match head {
            Some(index) => {
                let particle = self.fetched.remove(index);
                let result = read_args(
                    particle,
                    self.peer_id,
                    &mut self.local_vm.lock(),
                    &self.key_pair,
                );
                if let Some(result) = result {
                    result.map_err(|args| eyre!("AIR caught an error: {:?}", args))
                } else {
                    self.raw_wait_particle_args(particle_id).await
                }
            }
            None => self.raw_wait_particle_args(particle_id).await,
        }
    }

    async fn raw_wait_particle_args(&mut self, particle_id: impl AsRef<str>) -> Result<Vec<Value>> {
        let mut max = 100;
        loop {
            max -= 1;
            if max <= 0 {
                bail!("timed out waiting for particle {}", particle_id.as_ref());
            }
            let particle = self.raw_receive().await.ok();
            if let Some(particle) = particle {
                if particle.id == particle_id.as_ref() {
                    let result = read_args(
                        particle,
                        self.peer_id,
                        &mut self.local_vm.lock(),
                        &self.key_pair,
                    );
                    if let Some(result) = result {
                        break result.map_err(|args| eyre!("AIR caught an error: {:?}", args));
                    }
                } else {
                    self.fetched.push(particle)
                }
            }
        }
    }

    pub async fn listen_for_n<O: Default, F: Fn(Result<Vec<JValue>, Vec<JValue>>) -> O>(
        &mut self,
        mut n: usize,
        f: F,
    ) -> O {
        loop {
            n -= 1;
            if n == 0 {
                return O::default();
            }

            let particle = self.receive().await.ok();
            if let Some(particle) = particle {
                let args = read_args(
                    particle,
                    self.peer_id,
                    &mut self.local_vm.lock(),
                    &self.key_pair,
                );
                if let Some(args) = args {
                    return f(args);
                }
            }
        }
    }
}

pub async fn timeout<F, T>(dur: Duration, f: F) -> eyre::Result<T>
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(dur, f)
        .await
        .wrap_err(format!("timed out after {dur:?}"))
}
