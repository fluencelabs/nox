/*
 * Copyright 2019 Fluence Labs Limited
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

use libp2p::{
    core::{self, muxing::StreamMuxerBox, transport::boxed::Boxed},
    identity::Keypair,
    mplex::MplexConfig,
    secio::SecioConfig,
    tcp::TcpConfig,
    yamux::Config as YamuxConfig,
    PeerId, Transport,
};
use std::io::{Error, ErrorKind};
use std::time::Duration;

pub(crate) type PeerServiceTransport = Boxed<(PeerId, StreamMuxerBox), Error>;

/// Creates transport that is common for all connections.
///
/// Transport is based on TCP with SECIO as the encryption layer and MPLEX otr YAMUX as
/// the multiplexing layer.
pub fn build_transport(keys: Keypair, socket_timeout: Duration) -> PeerServiceTransport {
    TcpConfig::new()
        .nodelay(true)
        .upgrade(core::upgrade::Version::V1)
        .authenticate(SecioConfig::new(keys))
        .multiplex(
            core::upgrade::SelectUpgrade::<MplexConfig, YamuxConfig>::new(
                MplexConfig::default(),
                YamuxConfig::default(),
            ),
        )
        .map(|(peer, muxer), _| (peer, StreamMuxerBox::new(muxer)))
        .timeout(socket_timeout)
        .map_err(|err| Error::new(ErrorKind::Other, err))
        .boxed()
}
