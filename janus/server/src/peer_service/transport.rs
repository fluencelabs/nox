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

use libp2p::{core::muxing::StreamMuxer, identity::Keypair, secio::SecioConfig, PeerId, Transport};

use std::time::Duration;

/// Creates transport that is common for all connections.
///
/// Transport is based on Websocket over TCP with SECIO as the encryption layer and
/// MPLEX or YAMUX as the multiplexing layer.
pub fn build_transport(
    keys: Keypair,
    socket_timeout: Duration,
) -> impl Transport<
    Output = (
        PeerId,
        impl StreamMuxer<
                OutboundSubstream = impl Send,
                Substream = impl Send,
                Error = impl Into<std::io::Error>,
            > + Send
            + Sync,
    ),
    Error = impl std::error::Error + Send,
    Listener = impl Send,
    Dial = impl Send,
    ListenerUpgrade = impl Send,
> + Clone {
    let tcp = libp2p::tcp::TcpConfig::new().nodelay(true);
    let transport = libp2p::websocket::WsConfig::new(
        // libp2p::dns::DnsConfig::new(tcp).expect("Can't build DnsConfig"),
        tcp,
    );
    let secio = SecioConfig::new(keys);
    let mut yamux = libp2p::yamux::Config::default();

    yamux.set_max_num_streams(1024 * 1024);

    transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(secio)
        .multiplex(yamux)
        .timeout(socket_timeout)
}
