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

use crate::health::ConnectivityHealth;
use connection_pool::{ConnectionPoolApi, ConnectionPoolT, LifecycleEvent};
use fluence_libp2p::PeerId;
use futures::{stream::iter, StreamExt};
use humantime_serde::re::humantime::format_duration as pretty;
use kademlia::{KademliaApi, KademliaApiT, KademliaError};
use libp2p::Multiaddr;
use particle_protocol::{Contact, ExtendedParticle, SendStatus};
use peer_metrics::{ConnectivityMetrics, Resolution};
use tokio::time::sleep;
use tracing::{Instrument, instrument, Span};

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
    pub health: Option<ConnectivityHealth>,
}

impl Connectivity {
    pub fn start(self) -> Tasks {
        let reconnect_bootstraps = tokio::task::Builder::new()
            .name("reconnect_bootstraps")
            .spawn(self.clone().reconnect_bootstraps().in_current_span())
            .expect("Could not spawn task");
        let run_bootstrap = tokio::task::Builder::new()
            .name("run_bootstrap")
            .spawn(self.kademlia_bootstrap().in_current_span())
            .expect("Could not spawn task");

        Tasks::new("Connectivity", vec![run_bootstrap, reconnect_bootstraps])
    }

    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub async fn resolve_contact(&self, target: PeerId, particle_id: &str) -> Option<Contact> {
        let metrics = self.metrics.as_ref();
        let contact = self.connection_pool.get_contact(target).await;
        if let Some(contact) = contact {
            // contact is connected directly to current node
            if let Some(m) = metrics {
                m.count_resolution(Resolution::Local)
            }
            return Some(contact);
        } else {
            // contact isn't connected, have to discover it
            let contact = self.discover_peer(target).await;
            match contact {
                Ok(Some(contact)) => {
                    // connect to the discovered contact
                    let connected = self.connection_pool.connect(contact.clone()).await;
                    if connected {
                        if let Some(m) = metrics {
                            m.count_resolution(Resolution::Kademlia)
                        }
                        return Some(contact);
                    }
                    if let Some(m) = metrics {
                        m.count_resolution(Resolution::ConnectionFailed)
                    }
                    tracing::warn!(
                        particle_id = particle_id,
                        "{} Couldn't connect to {}",
                        self.peer_id,
                        target
                    );
                }
                Ok(None) => {
                    if let Some(m) = metrics {
                        m.count_resolution(Resolution::KademliaNotFound)
                    }
                    tracing::warn!(
                        particle_id = particle_id,
                        "{} Couldn't discover {}",
                        self.peer_id,
                        target
                    );
                }
                Err(err) => {
                    if let Some(m) = metrics {
                        m.count_resolution(Resolution::KademliaError)
                    }
                    let id = particle_id;
                    tracing::warn!(
                        particle_id = id,
                        "{} Failed to discover {}: {}",
                        self.peer_id,
                        target,
                        err
                    );
                }
            }
        };

        None
    }

    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub async fn send(&self, contact: Contact, particle: ExtendedParticle) -> bool {
        tracing::debug!(particle_id = particle.particle.id, "Sending particle to {}", contact);
        let metrics = self.metrics.as_ref();
        let id = particle.particle.id.clone();
        let sent = self.connection_pool.send(contact.clone(), particle).await;
        match &sent {
            SendStatus::Ok => {
                if let Some(m) = metrics {
                    m.send_particle_ok(&id)
                }
                tracing::info!(particle_id = id, "Sent particle to {}", contact);
            }
            err => {
                if let Some(m) = metrics {
                    m.send_particle_failed(&id);
                }
                tracing::warn!(
                    particle_id = id,
                    "Failed to send particle to {}, reason: {:?}",
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
            use tokio_stream::StreamExt as stream;
            let events = pool.lifecycle_events();
            stream::filter_map(events, move |e| {
                log::trace!(target: "network", "Connection pool event: {:?}", e);
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
        let health = self.health.as_ref();

        let disconnections = {
            use tokio_stream::StreamExt as stream;

            let bootstrap_nodes = bootstrap_nodes.clone();
            let events = pool.lifecycle_events();
            stream::filter_map(events, move |e| {
                if let LifecycleEvent::Disconnected(Contact { addresses, .. }) = e {
                    let addresses = addresses.into_iter();
                    let addresses = addresses.filter(|addr| bootstrap_nodes.contains(addr));
                    let addresses = addresses.collect::<Vec<_>>();
                    if !addresses.is_empty() {
                        metrics.map(|m| m.bootstrap_disconnected.inc());
                        if let Some(h) = health {
                            h.bootstrap_nodes
                                .on_bootstrap_disconnected(addresses.clone())
                        }
                    };
                    return Some(iter(addresses));
                }
                None
            })
        }
        .flatten();

        // TODO: take from config
        let max = Duration::from_secs(60);
        // TODO: exponential backoff + random?
        let delta = Duration::from_secs(5);

        let reconnect = move |kademlia: KademliaApi,
                              pool: ConnectionPoolApi,
                              addr: Multiaddr,
                              parent_span: Span| {
            (async move {
                let mut delay = Duration::from_secs(0);
                loop {
                    tracing::info!("Will reconnect bootstrap {}", addr);
                    if let Some(contact) = pool.dial(addr.clone()).await {
                        tracing::info!("Connected bootstrap {}", contact);
                        let ok = kademlia.add_contact(contact);
                        debug_assert!(ok, "kademlia.add_contact");
                        metrics.map(|m| m.bootstrap_connected.inc());
                        if let Some(h) = health {
                            h.bootstrap_nodes.on_bootstrap_connected(addr)
                        }
                        break;
                    }

                    delay = min(delay + delta, max);
                    log::warn!("can't connect bootstrap {} (pause {})", addr, pretty(delay));
                    sleep(delay).await;
                }
            })
            .instrument(parent_span)
        };

        let parent_span = tracing::Span::current();
        let bootstraps = iter(bootstrap_nodes.clone().into_iter().collect::<Vec<_>>());
        bootstraps
            .chain(disconnections)
            .for_each_concurrent(None, |addr| {
                reconnect(kademlia.clone(), pool.clone(), addr, parent_span.clone())
            })
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
