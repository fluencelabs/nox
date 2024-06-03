/*
 * Copyright 2024 Fluence DAO
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

use asynchronous_codec::{FramedRead, FramedWrite};
use std::fmt::Debug;
use std::{io, iter, time::Duration};

use futures::{
    future::BoxFuture, AsyncRead, AsyncWrite, AsyncWriteExt, FutureExt, SinkExt, StreamExt,
};
use libp2p::swarm::OneShotHandlerConfig;
use libp2p::{
    core::{InboundUpgrade, OutboundUpgrade, UpgradeInfo},
    swarm::OneShotHandler,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};

use crate::libp2p_protocol::codec::FluenceCodec;
use crate::{HandlerMessage, SendStatus, PROTOCOL_NAME};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ProtocolConfig {
    /// Timeout for applying the given upgrade on a substream
    #[serde(with = "humantime_serde", default = "default_upgrade_timeout")]
    pub upgrade_timeout: Duration,
    /// Timeout for outbound substream upgrades.
    #[serde(
        with = "humantime_serde",
        default = "default_outbound_substream_timeout"
    )]
    pub outbound_substream_timeout: Duration,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            upgrade_timeout: default_upgrade_timeout(),
            outbound_substream_timeout: default_outbound_substream_timeout(),
        }
    }
}

fn default_outbound_substream_timeout() -> Duration {
    Duration::from_secs(10)
}
fn default_upgrade_timeout() -> Duration {
    Duration::from_secs(10)
}

impl ProtocolConfig {
    pub fn new(upgrade_timeout: Duration, outbound_substream_timeout: Duration) -> Self {
        Self {
            upgrade_timeout,
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
                outbound_substream_timeout: item.outbound_substream_timeout,
                ..<_>::default()
            },
        )
    }
}

macro_rules! impl_upgrade_info {
    ($tname:ident) => {
        impl UpgradeInfo for $tname {
            type Info = &'static str;
            type InfoIter = iter::Once<Self::Info>;

            fn protocol_info(&self) -> Self::InfoIter {
                iter::once(PROTOCOL_NAME)
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
    type Error = std::io::Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, socket: Socket, _: Self::Info) -> Self::Future {
        async move {
            let msg = FramedRead::new(socket, FluenceCodec::new())
                .next()
                .await
                .ok_or(io::ErrorKind::UnexpectedEof)??;

            Ok(msg)
        }
        .map(|result| match result {
            Ok(msg) => {
                if log::log_enabled!(log::Level::Debug) {
                    log::debug!("Got inbound ProtocolMessage: {:?}", msg);
                } else {
                    log::info!("Got inbound ProtocolMessage: {}", msg);
                }
                let msg: HandlerMessage = msg.into();
                Ok(msg)
            }
            Err(err) => {
                log::warn!("Error processing inbound ProtocolMessage: {:?}", err);
                Err(err)
            }
        })
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
                FramedWrite::new(&mut socket, FluenceCodec::new())
                    .send(msg)
                    .await?;

                // WARNING: It is vitally important to ALWAYS close after all writes
                //          or some bytes may not be sent and it will lead to `unexpected EOF`
                //          error on InboundUpgrade side.
                //          See e.g. https://github.com/libp2p/rust-yamux/issues/117
                socket.close().await?;
                Ok(())
            };

            let result = write().await.map_err(|err| {
                log::warn!("Error sending ProtocolMessage: {:?}", err);
                err
            });

            if let Some(channel) = channel {
                // it's ok to ignore error here: inlet might be dropped any time
                let result = match &result {
                    Ok(_) => SendStatus::Ok,
                    Err(err) => SendStatus::ProtocolError(format!("{err:?}")),
                };
                channel.send(result).ok();
            }

            result
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use futures::prelude::*;
    use libp2p::core::transport::{ListenerId, TransportEvent};
    use libp2p::core::{
        multiaddr::multiaddr,
        transport::{memory::MemoryTransport, Transport},
    };
    use libp2p::{InboundUpgrade, OutboundUpgrade};
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

    #[tokio::test]
    async fn oneshot_channel_test() {
        let mem_addr = multiaddr![Memory(thread_rng().gen::<u64>())];
        let mut transport = MemoryTransport::new().boxed();
        let listener_id = ListenerId::next();
        transport.listen_on(listener_id, mem_addr).unwrap();

        let listener_addr = match transport.select_next_some().now_or_never() {
            Some(TransportEvent::NewAddress { listen_addr, .. }) => listen_addr,
            p => panic!("MemoryTransport not listening on an address!: {:?}", p),
        };

        let inbound = tokio::task::spawn(async move {
            let (listener_upgrade, _) = transport.select_next_some().await.into_incoming().unwrap();
            // let listener_event = poll_fn(|ctx| Pin::new(&mut transport).poll(ctx)).await;
            // let listener_event = listener.next().await.unwrap();
            // let (listener_upgrade, _) = listener_event.unwrap().into_upgrade().unwrap();
            let conn = listener_upgrade.await.unwrap();

            let config = ProtocolConfig::default();
            config.upgrade_inbound(conn, "/test/1").await.unwrap()
        });
        let msg: ProtocolMessage = serde_json::from_slice(&BYTES).unwrap();
        let sent_particle = match msg {
            ProtocolMessage::Particle(p) => p,
            _ => unreachable!("must be particle"),
        };
        let msg = HandlerMessage::OutParticle(sent_particle.clone(), <_>::default());
        let mut transport = MemoryTransport::new();
        let c = transport.dial(listener_addr).unwrap().await.unwrap();
        msg.upgrade_outbound(c, "/test/1").await.unwrap();
        let received_particle = inbound.await.unwrap();

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
