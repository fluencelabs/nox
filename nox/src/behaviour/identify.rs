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

use itertools::Itertools;
use libp2p::{
    core::{multiaddr::Protocol, Multiaddr},
    identify::Event as IdentifyEvent,
};
use particle_protocol::PROTOCOL_NAME;
use tokio::sync::oneshot;
use server_config::KademliaConfig;

use super::FluenceNetworkBehaviour;

/// Network address information is exchanged via Identify protocol.
/// That information is passed to relay, so nodes know each other's addresses
impl FluenceNetworkBehaviour {
    pub fn inject_identify_event(
        &mut self,
        config: &KademliaConfig,
        event: IdentifyEvent,
        allow_local_addresses: bool,
    ) {
        match event {
            IdentifyEvent::Received { peer_id, info, .. } => {
                log::trace!(
                    "Identify received from {}: protocols: {:?} version: {} listen addrs {:?}",
                    peer_id,
                    info.protocols,
                    info.protocol_version,
                    info.listen_addrs
                );

                let addresses = filter_addresses(info.listen_addrs.clone(), allow_local_addresses);

                let mut supports_kademlia = false;
                let mut supports_fluence = false;

                for protocol in info.protocols.iter() {
                    if !supports_kademlia && protocol.eq(&config.protocol_name) {
                        supports_kademlia = true;
                    }
                    if !supports_fluence && protocol.eq(&PROTOCOL_NAME) {
                        supports_fluence = true;
                    }
                    if supports_fluence && supports_kademlia {
                        break;
                    }
                }

                if supports_fluence {
                    let protocols: Vec<_> = info.protocols.iter().map(|p| p.to_string()).collect();
                    log::debug!(
                        target: "network",
                        "Found fluence peer {}: protocols: {:?} version: {} listen addrs {:?}",
                        peer_id, protocols, info.protocol_version, addresses
                    );
                    // Add addresses to connection pool disregarding whether it supports kademlia or not
                    // we want to have full info on non-kademlia peers as well
                    self.connection_pool
                        .add_discovered_addresses(peer_id, addresses.clone());
                    if supports_kademlia {
                        self.kademlia.add_kad_node(peer_id, addresses);
                    }
                } else {
                    log::debug!(
                        target: "blocked",
                        "Found peer {} not supported fluence protocol, protocols: {:?} version: {} listen addrs {:?}. skipping...",
                        peer_id, info.protocols,
                        info.protocol_version,
                        info.listen_addrs
                    );
                    let (out, _inlet) = oneshot::channel();
                    self.connection_pool.disconnect(peer_id, out);
                }
            }

            // TODO: handle error?
            IdentifyEvent::Error { error, peer_id } => {
                log::debug!("Identify error on {}: {}", peer_id, error);
            }

            // We don't care about outgoing identification info
            IdentifyEvent::Sent { .. } | IdentifyEvent::Pushed { .. } => {}
        }
    }
}

fn filter_addresses(addresses: Vec<Multiaddr>, allow_local: bool) -> Vec<Multiaddr> {
    // Deduplicate addresses
    let addresses = addresses.iter().unique();

    if allow_local {
        // Return all addresses
        addresses.cloned().collect()
    } else {
        // Keep only global addresses
        addresses
            .filter(|maddr| !is_local_maddr(maddr))
            .cloned()
            .collect()
    }
}

fn is_local_maddr(maddr: &Multiaddr) -> bool {
    maddr.iter().any(|p| match p {
        Protocol::Ip4(addr) => !addr.is_global(),
        Protocol::Ip6(addr) => !addr.is_global(),
        Protocol::Memory(_) => true,
        _ => false,
    })
}
