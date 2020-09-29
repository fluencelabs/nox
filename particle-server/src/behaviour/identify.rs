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

use super::ServerBehaviour;
use itertools::Itertools;
use libp2p::{
    core::{multiaddr::Protocol, Multiaddr},
    identify::IdentifyEvent,
    identity::PublicKey,
    swarm::NetworkBehaviourEventProcess,
};
use std::net::IpAddr;

/// Network address information is exchanged via Identify protocol.
/// That information is passed to relay, so nodes know each other's addresses
impl NetworkBehaviourEventProcess<IdentifyEvent> for ServerBehaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        match event {
            IdentifyEvent::Received { peer_id, info, .. } => {
                log::debug!(
                    "Identify received from {}: protocols: {:?} version: {} listen addrs {:?}",
                    peer_id,
                    info.protocols,
                    info.protocol_version,
                    info.listen_addrs
                );
                let supports_kademlia =
                    info.protocols.iter().any(|p| p.contains("/ipfs/kad/1.0.0"));
                match info.public_key {
                    PublicKey::Ed25519(public_key) if supports_kademlia => {
                        let addresses = filter_addresses(info.listen_addrs);
                        self.particle.add_kad_node(peer_id, addresses, public_key);
                    }
                    _ if supports_kademlia => {
                        log::error!(
                            "Unable to add node {} to kademlia, public key {:?} is not supported. \
                            Only ed25519 is supported. Will fallback to Direct routing.",
                            peer_id,
                            info.public_key
                        );
                    }
                    _ => {}
                }
            }

            // TODO: handle error?
            IdentifyEvent::Error { error, peer_id } => {
                log::debug!("Identify error on {}: {}", peer_id, error);
            }

            // We don't care about Sent identification info
            IdentifyEvent::Sent { .. } => {}
        }
    }
}

fn filter_addresses(addresses: Vec<Multiaddr>) -> Vec<Multiaddr> {
    // Deduplicate addresses
    let addresses: Vec<_> = addresses.into_iter().unique().collect();

    // Check if there's at least single global IP address
    let exists_global = addresses.iter().any(is_global_maddr);

    if !exists_global {
        log::debug!("No globally-reachable IP addresses found. Are we running on localhost?");
        // If there are no global addresses, we are most likely running locally
        // So take loopback address, and go with it.
        addresses.into_iter().filter(is_local_maddr).collect()
    } else {
        // Keep only global addresses
        addresses.into_iter().filter(is_global_maddr).collect()
    }
}

fn is_global(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(addr) => {
            !addr.is_private()
                && !addr.is_loopback()
                && !addr.is_link_local()
                && !addr.is_broadcast()
                && !addr.is_documentation()
                && !addr.is_unspecified()
        }
        IpAddr::V6(addr) => !addr.is_loopback() && !addr.is_unspecified(),
    }
}

fn is_global_maddr(maddr: &Multiaddr) -> bool {
    maddr.iter().any(|p| match p {
        Protocol::Ip4(addr) => is_global(addr.into()),
        _ => false,
    })
}

fn is_local_maddr(maddr: &Multiaddr) -> bool {
    maddr.iter().any(|p| match p {
        Protocol::Ip4(addr) if addr.is_loopback() => true,
        Protocol::Memory(_) => true,
        _ => false,
    })
}
