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

use crate::ProtocolMessage;

use futures::{future::BoxFuture, AsyncRead, AsyncWrite, AsyncWriteExt, FutureExt};
use libp2p::{
    core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo},
    swarm::{protocols_handler, OneShotHandler},
};
use serde::Deserialize;
use serde_json::json;
use std::{io, iter, time::Duration};

pub use failure::Error;
use libp2p::swarm::OneShotHandlerConfig;
use std::fmt::Debug;

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

    fn gen_error(&self, err: impl Debug) -> ProtocolMessage {
        ProtocolMessage::UpgradeError(json!({ "error": format!("{:?}", err) }))
    }
}

impl<OutProto: protocols_handler::OutboundUpgradeSend, OutEvent>
    Into<OneShotHandler<ProtocolConfig, OutProto, OutEvent>> for ProtocolConfig
{
    fn into(self) -> OneShotHandler<ProtocolConfig, OutProto, OutEvent> {
        OneShotHandler::new(
            protocols_handler::SubstreamProtocol::new(self.clone())
                .with_timeout(self.upgrade_timeout),
            OneShotHandlerConfig {
                keep_alive_timeout: self.keep_alive_timeout,
                outbound_substream_timeout: self.outbound_substream_timeout,
            },
        )
    }
}

// 100 Mb
#[allow(clippy::identity_op)]
const MAX_BUF_SIZE: usize = 100 * 1024 * 1024;
const PROTOCOL_INFO: &[u8] = b"/fluence/faas/1.0.0";

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
impl_upgrade_info!(ProtocolMessage);

impl<Socket> InboundUpgrade<Socket> for ProtocolConfig
where
    Socket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = ProtocolMessage;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, mut socket: Socket, info: Self::Info) -> Self::Future {
        async move {
            let process = async move |socket| -> Result<_, Error> {
                let packet = upgrade::read_one(socket, MAX_BUF_SIZE).await?;
                match std::str::from_utf8(&packet) {
                    Ok(str) => log::debug!("Got inbound ProtocolMessage: {}", str),
                    Err(err) => log::warn!("Can't parse inbound ProtocolMessage to UTF8 {}", err),
                }

                Ok(serde_json::from_slice(&packet)?)
            };

            match process(&mut socket).await {
                Ok(msg) => {
                    socket.close().await?;
                    Ok(msg)
                }
                Err(err) => {
                    log::warn!("Error processing inbound ProtocolMessage: {:?}", err);
                    // Generate and send error back through socket
                    let err_msg = self.gen_error(&err);
                    err_msg.upgrade_outbound(socket, info).await?;
                    Err(err)
                }
            }
        }
        .boxed()
    }
}

impl<Socket> OutboundUpgrade<Socket> for ProtocolMessage
where
    Socket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = ();
    type Error = io::Error;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, mut socket: Socket, _: Self::Info) -> Self::Future {
        async move {
            match serde_json::to_string(&self) {
                Ok(str) => log::debug!("Sending outbound ProtocolMessage: {}", str),
                Err(err) => log::warn!("Can't serialize {:?} to string {}", &self, err),
            }

            let write = async move || -> Result<_, io::Error> {
                let bytes = serde_json::to_vec(&self)?;
                upgrade::write_one(&mut socket, bytes).await?;
                Ok(())
            };

            write().await.map_err(|err| {
                log::warn!("Error sending outbound ProtocolMessage: {:?}", err);
                err
            })
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::ProtocolMessage;
    use futures::prelude::*;
    use libp2p::core::{
        multiaddr::multiaddr,
        transport::{memory::MemoryTransport, ListenerEvent, Transport},
        upgrade,
    };

    use crate::ProtocolConfig;
    use rand::{thread_rng, Rng};

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

        let sent_call = async_std::task::block_on(async move {
            let call = ProtocolMessage::Particle(<_>::default());
            let c = MemoryTransport.dial(listener_addr).unwrap().await.unwrap();
            upgrade::apply_outbound(c, call.clone(), upgrade::Version::V1)
                .await
                .unwrap();
            call
        });

        let received_call = futures::executor::block_on(inbound);

        assert_eq!(sent_call, received_call);
    }
}
