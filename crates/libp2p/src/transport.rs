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
use futures_util::{AsyncReadExt, AsyncWriteExt};
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::{Boxed, MemoryTransport};
use libp2p::core::Multiaddr;
use libp2p::dns::tokio::Transport as TokioDnsConfig;
use libp2p::tcp::Transport as TcpTransport;
use libp2p::tcp::{tokio::Tcp as TokioTcp, Config as GenTcpConfig};
use libp2p::{core, identity::Keypair, PeerId, Transport as NetworkTransport};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::Level;

pub fn build_transport(
    transport: Transport,
    key_pair: &Keypair,
    timeout: Duration,
    network_key: NetworkKey,
) -> Boxed<(PeerId, StreamMuxerBox)> {
    match transport {
        Transport::Network => build_network_transport(key_pair, timeout, network_key),
        Transport::Memory => build_memory_transport(key_pair, timeout, network_key),
    }
}

/// Creates transport that is common for all connections.
///
/// Transport is based on TCP with SECIO as the encryption layer and MPLEX otr YAMUX as
/// the multiplexing layer.
pub fn build_network_transport(
    key_pair: &Keypair,
    socket_timeout: Duration,
    network_key: NetworkKey,
) -> Boxed<(PeerId, StreamMuxerBox)> {
    let tcp = || {
        let tcp = TcpTransport::<TokioTcp>::new(GenTcpConfig::default().nodelay(true));

        TokioDnsConfig::system(tcp).expect("Can't build DNS")
    };

    let transport = {
        let mut websocket = libp2p::websocket::WsConfig::new(tcp());
        websocket.set_tls_config(libp2p::websocket::tls::Config::client());
        websocket.or_transport(tcp())
    };

    configure_transport(transport, key_pair, socket_timeout, network_key)
}

#[derive(Error, Debug)]
pub enum HandshakeError {
    #[error("Failed to flush the data to the socket")]
    FailedFlush,
    #[error("Failed to write the network key to the socket")]
    FailedWriteKey,
    #[error("Failed to read the network key from the socket")]
    FailedReadKey,
    #[error("Wrong network key")]
    WrongKey,
}

#[derive(Clone, Debug)]
pub struct NetworkKey([u8; 32]);

impl NetworkKey {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let mut res: [u8; 32] = Default::default();
        rng.fill_bytes(&mut res);
        NetworkKey(res)
    }
}

impl From<[u8; 32]> for NetworkKey {
    fn from(value: [u8; 32]) -> Self {
        NetworkKey(value)
    }
}

impl From<NetworkKey> for [u8; 32] {
    fn from(value: NetworkKey) -> Self {
        value.0
    }
}

impl NetworkKey {
    pub async fn handshake<TSocket>(self, mut socket: TSocket) -> Result<TSocket, HandshakeError>
    where
        TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let mut remote_key = vec![0; self.0.len()];

        tracing::trace!("exchanging nonces");
        socket
            .write_all(&self.0)
            .await
            .map_err(|_| HandshakeError::FailedWriteKey)?;
        socket
            .flush()
            .await
            .map_err(|_| HandshakeError::FailedFlush)?;

        socket
            .read_exact(&mut remote_key)
            .await
            .map_err(|_| HandshakeError::FailedReadKey)?;

        if remote_key != self.0 {
            if tracing::enabled!(Level::TRACE) {
                let expected = hex::encode(&self.0);
                let remote = hex::encode(&remote_key);
                tracing::trace!(
                    "Failed to match expected and remote nonces {:?} {:?}",
                    expected,
                    remote
                );
            }
            return Err(HandshakeError::WrongKey);
        }
        Ok(socket)
    }
}

pub fn configure_transport<T, C>(
    transport: T,
    key_pair: &Keypair,
    transport_timeout: Duration,
    network_key: NetworkKey,
) -> Boxed<(PeerId, StreamMuxerBox)>
where
    T: NetworkTransport<Output = C> + Send + Sync + Unpin + 'static,
    C: AsyncRead + AsyncWrite + Unpin + Send + Unpin + 'static,
    T::Dial: Send + Unpin + 'static,
    T::ListenerUpgrade: Send + Unpin + 'static,
    T::Error: Send + Unpin + Sync + 'static,
{
    let multiplex = {
        let mut mplex = libp2p_mplex::MplexConfig::default();
        mplex.set_max_num_streams(1024 * 1024);

        let mut yamux = libp2p::yamux::Config::default();
        yamux.set_max_num_streams(1024 * 1024);

        core::upgrade::SelectUpgrade::new(yamux, mplex)
    };

    let auth_config = libp2p::noise::Config::new(key_pair).expect("create noise keypair");

    transport
        .and_then(move |socket, _| network_key.handshake(socket))
        .upgrade(core::upgrade::Version::V1)
        .authenticate(auth_config)
        .multiplex(multiplex)
        .timeout(transport_timeout)
        .boxed()
}

pub fn build_memory_transport(
    key_pair: &Keypair,
    transport_timeout: Duration,
    network_key: NetworkKey,
) -> Boxed<(PeerId, StreamMuxerBox)> {
    let transport = MemoryTransport::default();

    configure_transport(transport, key_pair, transport_timeout, network_key)
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
