/*
 * Copyright 2021 Fluence Labs Limited
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

use std::cmp::min;
use std::collections::HashSet;
use std::time::Duration;

use async_std::task::{sleep, spawn};
use futures::{stream::iter, StreamExt};
use humantime_serde::re::humantime::format_duration as pretty;
use libp2p::Multiaddr;

use connection_pool::{ConnectionPoolApi, ConnectionPoolT, LifecycleEvent};
use fluence_libp2p::PeerId;
use kademlia::{KademliaApi, KademliaApiT, KademliaError};
use particle_protocol::{Contact, Particle, SendStatus};
use peer_metrics::{ConnectivityMetrics, Resolution};

use crate::tasks::Tasks;

#[derive(Clone)]
/// This structure is just a composition of Kademlia and ConnectionPool.
/// It exists solely for code conciseness (i.e. avoid tuples);
/// there's no architectural motivation behind
pub struct Connectivity {
    pub peer_id: PeerId,
    pub kademlia: KademliaApi,
    pub connection_pool: ConnectionPoolApi,
    pub bootstrap_nodes: HashSet<Multiaddr>,
    /// Bootstrap will be executed after [1, N, 2*N, 3*N, ...] bootstrap nodes connected
    /// This setting specify that N.
    pub bootstrap_frequency: usize,
    pub metrics: Option<ConnectivityMetrics>,
}

impl Connectivity {
    pub fn start(self) -> Tasks {
        let reconnect_bootstraps = spawn(self.clone().reconnect_bootstraps());
        let run_bootstrap = spawn(self.kademlia_bootstrap());

        Tasks::new("Connectivity", vec![run_bootstrap, reconnect_bootstraps])
    }

    pub async fn resolve_contact(&self, target: PeerId, particle_id: &str) -> Option<Contact> {
        let metrics = self.metrics.as_ref();
        let contact = self.connection_pool.get_contact(target).await;
        if let Some(contact) = contact {
            // contact is connected directly to current node
            metrics.map(|m| m.count_resolution(Resolution::Local));
            return Some(contact);
        } else {
            // contact isn't connected, have to discover it
            let contact = self.discover_peer(target).await;
            match contact {
                Ok(Some(contact)) => {
                    // connect to the discovered contact
                    let connected = self.connection_pool.connect(contact.clone()).await;
                    if connected {
                        metrics.map(|m| m.count_resolution(Resolution::Kademlia));
                        return Some(contact);
                    }
                    metrics.map(|m| m.count_resolution(Resolution::ConnectionFailed));
                    log::warn!(
                        "{} Couldn't connect to {} for particle {}",
                        self.peer_id,
                        target,
                        particle_id
                    );
                }
                Ok(None) => {
                    metrics.map(|m| m.count_resolution(Resolution::KademliaNotFound));
                    log::warn!(
                        "{} Couldn't discover {} for particle {}",
                        self.peer_id,
                        target,
                        particle_id
                    );
                }
                Err(err) => {
                    metrics.map(|m| m.count_resolution(Resolution::KademliaError));
                    let id = particle_id;
                    log::warn!(
                        "{} Failed to discover {} for particle {}: {}",
                        self.peer_id,
                        target,
                        id,
                        err
                    );
                }
            }
        };

        None
    }

    pub async fn send(&self, contact: Contact, particle: Particle) -> bool {
        log::debug!("Sending particle {} to {}", particle.id, contact);
        let metrics = self.metrics.as_ref();
        let id = particle.id.clone();
        let sent = self.connection_pool.send(contact.clone(), particle).await;
        match &sent {
            SendStatus::Ok => {
                metrics.map(|m| m.particle_send_success.inc());
                log::info!("Sent particle {} to {}", id, contact);
            }
            err => {
                metrics.map(|m| m.particle_send_failure.inc());
                log::warn!(
                    "Failed to send particle {} to {}, reason: {:?}",
                    id,
                    contact,
                    err
                )
            }
        }

        matches!(sent, SendStatus::Ok)
    }

    /// Discover a peer via Kademlia
    pub async fn discover_peer(&self, target: PeerId) -> Result<Option<Contact>, KademliaError> {
        // discover contact addresses through Kademlia
        let addresses = self.kademlia.discover_peer(target).await?;
        if addresses.is_empty() {
            return Ok(None);
        }

        Ok(Some(Contact::new(target, addresses)))
    }

    /// Run kademlia bootstrap after first bootstrap is connected, and then every `frequency`
    pub async fn kademlia_bootstrap(self) {
        let kademlia = self.kademlia;
        let pool = self.connection_pool;
        let bootstrap_nodes = self.bootstrap_nodes;
        let frequency = self.bootstrap_frequency;

        // Count connected (and reconnected) bootstrap nodes
        let connections = {
            use async_std::stream::StreamExt as stream;

            let events = pool.lifecycle_events();
            stream::filter_map(events, move |e| {
                if let LifecycleEvent::Connected(c) = e {
                    let mut addresses = c.addresses.iter();
                    addresses.find(|addr| bootstrap_nodes.contains(addr))?;
                    return Some(c);
                }
                None
            })
        }
        .enumerate();

        connections
            .for_each(move |(n, contact)| {
                let kademlia = kademlia.clone();
                async move {
                    if n % frequency == 0 {
                        kademlia.add_contact(contact);
                        if let Err(err) = kademlia.bootstrap().await {
                            log::warn!("Kademlia bootstrap failed: {}", err)
                        } else {
                            log::info!("Kademlia bootstrap finished");
                        }
                    }
                }
            })
            .await;
    }

    /// Dial bootstraps, and then re-dial on each disconnection
    pub async fn reconnect_bootstraps(self) {
        let pool = self.connection_pool;
        let kademlia = self.kademlia;
        let bootstrap_nodes = self.bootstrap_nodes;
        let metrics = self.metrics.as_ref();

        let disconnections = {
            use async_std::stream::StreamExt as stream;

            let bootstrap_nodes = bootstrap_nodes.clone();
            let events = pool.lifecycle_events();
            stream::filter_map(events, move |e| {
                if let LifecycleEvent::Disconnected(Contact { addresses, .. }) = e {
                    metrics.map(|m| m.bootstrap_disconnected.inc());
                    let addresses = addresses.into_iter();
                    let addresses = addresses.filter(|addr| bootstrap_nodes.contains(addr));
                    let addresses = iter(addresses.collect::<Vec<_>>());
                    return Some(addresses);
                }
                None
            })
        }
        .flatten();

        // TODO: take from config
        let max = Duration::from_secs(60);
        // TODO: exponential backoff + random?
        let delta = Duration::from_secs(5);

        let reconnect = move |kademlia: KademliaApi, pool: ConnectionPoolApi, addr: Multiaddr| async move {
            let mut delay = Duration::from_secs(0);
            loop {
                if let Some(contact) = pool.dial(addr.clone()).await {
                    log::info!("Connected bootstrap {}", contact);
                    let ok = kademlia.add_contact(contact);
                    debug_assert!(ok, "kademlia.add_contact");
                    metrics.map(|m| m.bootstrap_connected.inc());
                    break;
                }

                delay = min(delay + delta, max);
                log::warn!("can't connect bootstrap {} (pause {})", addr, pretty(delay));
                sleep(delay).await;
            }
        };

        let bootstraps = iter(bootstrap_nodes.clone().into_iter().collect::<Vec<_>>());
        bootstraps
            .chain(disconnections)
            .for_each_concurrent(None, |addr| reconnect(kademlia.clone(), pool.clone(), addr))
            .await;
    }
}

impl AsRef<KademliaApi> for Connectivity {
    fn as_ref(&self) -> &KademliaApi {
        &self.kademlia
    }
}

impl AsRef<ConnectionPoolApi> for Connectivity {
    fn as_ref(&self) -> &ConnectionPoolApi {
        &self.connection_pool
    }
}
