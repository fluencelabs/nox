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

use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::Boxed;
use libp2p::noise;
use libp2p::{core, dns, identity::Keypair, PeerId, Transport};
use std::time::Duration;

/// Creates transport that is common for all connections.
///
/// Transport is based on TCP with SECIO as the encryption layer and MPLEX otr YAMUX as
/// the multiplexing layer.
pub fn build_transport(
    key_pair: Keypair,
    socket_timeout: Duration,
) -> Boxed<(PeerId, StreamMuxerBox)> {
    let multiplex = {
        let mut mplex = libp2p::mplex::MplexConfig::default();
        mplex.set_max_num_streams(1024 * 1024);
        let mut yamux = libp2p::yamux::YamuxConfig::default();
        yamux.set_max_num_streams(1024 * 1024);
        core::upgrade::SelectUpgrade::new(yamux, mplex)
    };

    let transport = {
        let tcp = libp2p::tcp::TcpConfig::new().nodelay(true);
        let tcp = dns::DnsConfig::new(tcp).expect("Can't build DNS");
        let websocket = libp2p::websocket::WsConfig::new(tcp.clone());
        tcp.or_transport(websocket)
    };

    let keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&key_pair)
        .expect("create noise keypair");
    let auth = libp2p::noise::NoiseConfig::xx(keys);

    transport
        .upgrade(core::upgrade::Version::V1)
        .authenticate(auth.into_authenticated())
        .multiplex(multiplex)
        .timeout(socket_timeout)
        .boxed()
}

pub fn build_memory_transport(key_pair: Keypair) -> Boxed<(PeerId, StreamMuxerBox)> {
    use libp2p::{
        core::{transport::MemoryTransport, upgrade},
        plaintext::PlainText2Config,
        yamux,
    };
    MemoryTransport::default()
        .upgrade(upgrade::Version::V1)
        .authenticate(PlainText2Config {
            local_public_key: key_pair.public(),
        })
        .multiplex(yamux::YamuxConfig::default())
        .map(|(p, m), _| (p, StreamMuxerBox::new(m)))
        .boxed()
}
