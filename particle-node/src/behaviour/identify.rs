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
    identify::IdentifyEvent,
};

use super::NetworkBehaviour;

/// Network address information is exchanged via Identify protocol.
/// That information is passed to relay, so nodes know each other's addresses
impl NetworkBehaviour {
    fn inject_identify_event(&mut self, event: IdentifyEvent, allow_local_addresses: bool) {
        match event {
            IdentifyEvent::Received { peer_id, info, .. } => {
                log::trace!(
                    "Identify received from {}: protocols: {:?} version: {} listen addrs {:?}",
                    peer_id,
                    info.protocols,
                    info.protocol_version,
                    info.listen_addrs
                );

                let addresses = filter_addresses(info.listen_addrs, allow_local_addresses);

                // Add addresses to connection pool disregarding whether it supports kademlia or not
                // we want to have full info on non-kademlia peers as well
                self.connection_pool
                    .add_discovered_addresses(peer_id, addresses.clone());

                let supports_kademlia =
                    info.protocols.iter().any(|p| p.contains("/ipfs/kad/1.0.0"));
                if supports_kademlia {
                    self.kademlia.add_kad_node(peer_id, addresses);
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
    let addresses: Vec<_> = addresses.into_iter().unique().collect();

    if allow_local {
        // Return all addresses
        addresses
    } else {
        // Keep only global addresses
        addresses
            .into_iter()
            .filter(|maddr| !is_local_maddr(maddr))
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
