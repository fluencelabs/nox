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

use std::time::Duration;

use futures::{AsyncRead, AsyncWrite};
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::{Boxed, MemoryTransport};
use libp2p::core::Multiaddr;
use libp2p::noise;
use libp2p::{core, dns, identity::Keypair, PeerId, Transport as NetworkTransport};
use serde::{Deserialize, Serialize};

pub fn build_transport(
    transport: Transport,
    key_pair: Keypair,
    timeout: Duration,
    split_size: usize,
) -> Boxed<(PeerId, StreamMuxerBox)> {
    match transport {
        Transport::Network => build_network_transport(key_pair, timeout, split_size),
        Transport::Memory => build_memory_transport(key_pair, timeout, split_size),
    }
}

/// Creates transport that is common for all connections.
///
/// Transport is based on TCP with SECIO as the encryption layer and MPLEX otr YAMUX as
/// the multiplexing layer.
pub fn build_network_transport(
    key_pair: Keypair,
    socket_timeout: Duration,
    split_size: usize,
) -> Boxed<(PeerId, StreamMuxerBox)> {
    let transport = {
        let tcp = libp2p::tcp::TcpConfig::new().nodelay(true);
        // TODO: expose async?
        let tcp = async_std::task::block_on(dns::DnsConfig::system(tcp)).expect("Can't build DNS");
        let mut websocket = libp2p::websocket::WsConfig::new(tcp.clone());
        websocket.set_tls_config(libp2p::websocket::tls::Config::client());
        websocket.or_transport(tcp)
    };

    configure_transport(transport, key_pair, socket_timeout, split_size)
}

pub fn configure_transport<T, C>(
    transport: T,
    key_pair: Keypair,
    transport_timeout: Duration,
    split_size: usize,
) -> Boxed<(PeerId, StreamMuxerBox)>
where
    T: NetworkTransport<Output = C> + Send + Sync + 'static,
    C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    T::Dial: Send + 'static,
    T::Listener: Send + 'static,
    T::ListenerUpgrade: Send + 'static,
    T::Error: Send + Sync + 'static,
{
    let multiplex = {
        let mut mplex = libp2p::mplex::MplexConfig::default();
        mplex.set_max_num_streams(1024 * 1024);
        mplex.set_split_send_size(split_size);

        let mut yamux = libp2p::yamux::YamuxConfig::default();
        yamux.set_max_num_streams(1024 * 1024);
        yamux.set_split_send_size(split_size);

        core::upgrade::SelectUpgrade::new(yamux, mplex)
    };

    let keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&key_pair)
        .expect("create noise keypair");
    let auth = libp2p::noise::NoiseConfig::xx(keys);

    transport
        .upgrade(core::upgrade::Version::V1)
        .authenticate(auth.into_authenticated())
        .multiplex(multiplex)
        .timeout(transport_timeout)
        .boxed()
}

pub fn build_memory_transport(
    key_pair: Keypair,
    transport_timeout: Duration,
    split_size: usize,
) -> Boxed<(PeerId, StreamMuxerBox)> {
    let transport = MemoryTransport::default();

    configure_transport(transport, key_pair, transport_timeout, split_size)
}

#[derive(Clone, Debug, Deserialize, Serialize, Copy)]
pub enum Transport {
    Memory,
    Network,
}

impl Transport {
    pub fn is_network(&self) -> bool {
        matches!(self, Transport::Network)
    }

    pub fn from_maddr(maddr: &Multiaddr) -> Self {
        use libp2p::core::multiaddr::Protocol::Memory;
        if maddr.iter().any(|p| matches!(p, Memory(_))) {
            Transport::Memory
        } else {
            Transport::Network
        }
    }
}
