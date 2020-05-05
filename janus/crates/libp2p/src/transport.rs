/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use libp2p::{
    core::{self, muxing::StreamMuxer},
    dns,
    identity::Keypair,
    secio::SecioConfig,
    PeerId, Transport,
};
use std::time::Duration;

/// Creates transport that is common for all connections.
///
/// Transport is based on TCP with SECIO as the encryption layer and MPLEX otr YAMUX as
/// the multiplexing layer.
pub fn build_transport(
    key_pair: Keypair,
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
    let multiplex = {
        let mut mplex = libp2p::mplex::MplexConfig::default();
        mplex.max_substreams(1024 * 1024);
        let mut yamux = libp2p::yamux::Config::default();
        yamux.set_max_num_streams(1024 * 1024);
        core::upgrade::SelectUpgrade::new(yamux, mplex)
    };
    let secio = SecioConfig::new(key_pair);

    let transport = {
        let tcp = libp2p::tcp::TcpConfig::new().nodelay(true);
        let tcp = dns::DnsConfig::new(tcp).expect("Can't build DNS");
        let websocket = libp2p::websocket::WsConfig::new(tcp.clone());
        tcp.or_transport(websocket)
    };

    transport
        .upgrade(core::upgrade::Version::V1)
        .authenticate(secio)
        .multiplex(multiplex)
        .timeout(socket_timeout)
}
