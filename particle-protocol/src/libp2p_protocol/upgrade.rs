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

use std::fmt::Debug;
use std::{io, iter, time::Duration};

pub use eyre::Error;
use eyre::WrapErr;
use futures::{future::BoxFuture, AsyncRead, AsyncReadExt, AsyncWrite, FutureExt};
use libp2p::swarm::OneShotHandlerConfig;
use libp2p::{
    core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo},
    swarm::OneShotHandler,
};
use log::LevelFilter;
use serde::Deserialize;

use crate::libp2p_protocol::message::ProtocolMessage;
use crate::{HandlerMessage, PROTOCOL_NAME};

// TODO: embed pings into the protocol?
// TODO: embed identify into the protocol?
#[derive(Clone, Deserialize, Debug)]
pub struct ProtocolConfig {
    /// Timeout for applying the given upgrade on a substream
    #[serde(with = "humantime_serde")]
    pub upgrade_timeout: Duration,
    /// Keep-alive timeout for idle connections.
    #[serde(with = "humantime_serde")]
    pub keep_alive_timeout: Duration,
    /// Timeout for outbound substream upgrades.
    #[serde(with = "humantime_serde")]
    pub outbound_substream_timeout: Duration,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            upgrade_timeout: Duration::from_secs(10),
            keep_alive_timeout: Duration::from_secs(10),
            outbound_substream_timeout: Duration::from_secs(10),
        }
    }
}

impl ProtocolConfig {
    pub fn new(
        upgrade_timeout: Duration,
        keep_alive_timeout: Duration,
        outbound_substream_timeout: Duration,
    ) -> Self {
        Self {
            upgrade_timeout,
            keep_alive_timeout,
            outbound_substream_timeout,
        }
    }
}

impl<OutProto: libp2p::swarm::handler::OutboundUpgradeSend, OutEvent> From<ProtocolConfig>
    for OneShotHandler<ProtocolConfig, OutProto, OutEvent>
{
    fn from(item: ProtocolConfig) -> OneShotHandler<ProtocolConfig, OutProto, OutEvent> {
        OneShotHandler::new(
            libp2p::swarm::handler::SubstreamProtocol::new(item.clone(), ())
                .with_timeout(item.upgrade_timeout),
            OneShotHandlerConfig {
                keep_alive_timeout: item.keep_alive_timeout,
                outbound_substream_timeout: item.outbound_substream_timeout,
                ..<_>::default()
            },
        )
    }
}

// 100 Mb
#[allow(clippy::identity_op)]
const MAX_BUF_SIZE: usize = 100 * 1024 * 1024;
const PROTOCOL_INFO: &[u8] = PROTOCOL_NAME.as_bytes();

macro_rules! impl_upgrade_info {
    ($tname:ident) => {
        impl UpgradeInfo for $tname {
            type Info = &'static [u8];
            type InfoIter = iter::Once<Self::Info>;

            fn protocol_info(&self) -> Self::InfoIter {
                iter::once(PROTOCOL_INFO)
            }
        }
    };
}

impl_upgrade_info!(ProtocolConfig);
impl_upgrade_info!(HandlerMessage);

impl<Socket> InboundUpgrade<Socket> for ProtocolConfig
where
    Socket: AsyncRead + Send + Unpin + 'static,
{
    type Output = HandlerMessage;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: Socket, _: Self::Info) -> Self::Future {
        async move {
            let process = async move |mut socket: Socket| -> Result<ProtocolMessage, Error> {
                let mut packet = Vec::new();
                let read_result = socket.read_to_end(&mut packet).await;
                if let Err(err) = read_result {
                    log::warn!(target: "network", "read_to_end error: {:?}, buffer {:?}", err, packet);
                }

                let decoded = upgrade::read_length_prefixed(&mut packet.as_slice(), MAX_BUF_SIZE).await;

                match decoded {
                    Ok(decoded) => {
                        serde_json::from_slice(&decoded)
                            .wrap_err_with(|| format!("unable to deserialize: '{:?}'", packet))
                    },
                    Err(err) => {
                        let expected_len = upgrade::read_varint(&mut packet.as_slice()).await?;
                        log::warn!(target: "network", "read_length_prefixed error: {:?}, original buffer {:?}\nexpected length {}, len with prefix {}", err, packet, expected_len, packet.len());
                        Err(err.into())
                    }
                }
            };

            match process(socket).await {
                Ok(msg) => {
                    if log::log_enabled!(log::Level::Debug) {
                        log::debug!("Got inbound ProtocolMessage: {:?}", msg);
                    } else {
                        log::info!("Got inbound ProtocolMessage: {}", msg);
                    }
                    Ok(msg.into())
                }
                Err(err) => {
                    log::warn!("Error processing inbound ProtocolMessage: {:?}", err);
                    Err(err)
                }
            }
        }
        .boxed()
    }
}

impl<Socket> OutboundUpgrade<Socket> for HandlerMessage
where
    Socket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = ();
    type Error = io::Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, mut socket: Socket, _: Self::Info) -> Self::Future {
        async move {
            let (msg, channel) = self.into_protocol_message();

            if log::max_level() >= LevelFilter::Debug {
                match serde_json::to_string(&msg) {
                    Ok(str) => log::debug!("Sending ProtocolMessage: {}", str),
                    Err(err) => log::warn!("Can't serialize {:?} to string {}", &msg, err),
                }
            }

            let write = async move || -> Result<_, io::Error> {
                let bytes = serde_json::to_vec(&msg)?;
                upgrade::write_length_prefixed(&mut socket, bytes).await?;
                Ok(())
            };

            let result = write().await.map_err(|err| {
                log::warn!("Error sending ProtocolMessage: {:?}", err);
                err
            });

            if let Some(channel) = channel {
                // it's ok to ignore error here: inlet might be dropped any time
                channel.send(result.is_ok()).ok();
            }

            result
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use futures::prelude::*;
    use libp2p::core::{
        multiaddr::multiaddr,
        transport::{memory::MemoryTransport, ListenerEvent, Transport},
        upgrade,
    };
    use rand::{thread_rng, Rng};

    use crate::libp2p_protocol::message::ProtocolMessage;
    use crate::{HandlerMessage, ProtocolConfig};

    const BYTES: [u8; 175] = [
        123, 34, 97, 99, 116, 105, 111, 110, 34, 58, 34, 80, 97, 114, 116, 105, 99, 108, 101, 34,
        44, 34, 105, 100, 34, 58, 34, 49, 34, 44, 34, 105, 110, 105, 116, 95, 112, 101, 101, 114,
        95, 105, 100, 34, 58, 34, 49, 50, 68, 51, 75, 111, 111, 87, 67, 74, 104, 76, 98, 78, 51,
        118, 67, 101, 112, 109, 70, 106, 114, 87, 70, 53, 90, 70, 68, 71, 65, 117, 65, 89, 86, 121,
        78, 74, 51, 70, 49, 49, 101, 80, 99, 119, 76, 76, 82, 120, 86, 76, 34, 44, 34, 116, 105,
        109, 101, 115, 116, 97, 109, 112, 34, 58, 49, 54, 49, 55, 55, 51, 55, 48, 49, 54, 57, 51,
        49, 44, 34, 116, 116, 108, 34, 58, 54, 53, 53, 50, 53, 44, 34, 115, 99, 114, 105, 112, 116,
        34, 58, 34, 34, 44, 34, 115, 105, 103, 110, 97, 116, 117, 114, 101, 34, 58, 91, 93, 44, 34,
        100, 97, 116, 97, 34, 58, 34, 34, 125,
    ];

    #[test]
    fn oneshot_channel_test() {
        let mem_addr = multiaddr![Memory(thread_rng().gen::<u64>())];
        let mut listener = MemoryTransport.listen_on(mem_addr).unwrap();
        let listener_addr =
            if let Some(Some(Ok(ListenerEvent::NewAddress(a)))) = listener.next().now_or_never() {
                a
            } else {
                panic!("MemoryTransport not listening on an address!");
            };

        let inbound = async_std::task::spawn(async move {
            let listener_event = listener.next().await.unwrap();
            let (listener_upgrade, _) = listener_event.unwrap().into_upgrade().unwrap();
            let conn = listener_upgrade.await.unwrap();
            let config = ProtocolConfig::default();
            upgrade::apply_inbound(conn, config).await.unwrap()
        });

        let sent_particle = async_std::task::block_on(async move {
            let msg: ProtocolMessage = serde_json::from_slice(&BYTES).unwrap();
            let particle = match msg {
                ProtocolMessage::Particle(p) => p,
                _ => unreachable!("must be particle"),
            };
            let msg = HandlerMessage::OutParticle(particle.clone(), <_>::default());
            let c = MemoryTransport.dial(listener_addr).unwrap().await.unwrap();
            upgrade::apply_outbound(c, msg, upgrade::Version::V1)
                .await
                .unwrap();
            particle
        });

        let received_particle = futures::executor::block_on(inbound);

        match received_particle {
            HandlerMessage::InParticle(received_particle) => {
                assert_eq!(sent_particle, received_particle)
            }
            _ => unreachable!("must be InParticle"),
        }
    }

    #[test]
    fn deserialize() {
        let str = r#"{"action":"Particle","id":"2","init_peer_id":"12D3KooWAcn1f5iZ7wbo9QrYPFgq6o7DGkh7VwC8Zucn6DgWZQDo","timestamp":1617733422130,"ttl":65525,"script":"!","signature":[],"data":"MTJEM0tvb1dDM3dhcjhqcTJzaGFVQ2hSZWttYjNNN0RGRGl4ZkdVTm5ydGY0VlRGQVlVdywxMkQzS29vV0o2bVZLYXpKQzdyd2dtd0JpZm5LZ0JoR2NSTWtaOXdRTjY4dmJ1UGdIUjlO"}"#;

        let _: ProtocolMessage = serde_json::from_str(str).unwrap();

        let test_msg: Result<ProtocolMessage, _> = serde_json::from_slice(&BYTES);
        test_msg.unwrap();
    }
}
