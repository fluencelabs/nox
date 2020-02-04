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

use crate::error::Error;
use crate::node_service::relay::events::RelayEvent;
use futures::{AsyncRead, AsyncWrite, AsyncWriteExt, Future};
use libp2p::core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use log::trace;
use serde_json;
use std::{io, iter, pin::Pin};

// 1 Mb
const MAX_BUF_SIZE: usize = 1 * 1024 * 1024;
const PROTOCOL_INFO: &[u8] = b"/janus/relay/1.0.0";

impl UpgradeInfo for RelayEvent {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(PROTOCOL_INFO)
    }
}

impl<Socket> InboundUpgrade<Socket> for RelayEvent
where
    Socket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = RelayEvent;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, mut socket: Socket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            let packet = upgrade::read_one(&mut socket, MAX_BUF_SIZE).await?;
            let relay_event: RelayEvent = serde_json::from_slice(&packet).unwrap();
            socket.close().await?;

            Ok(relay_event)
        })
    }
}

impl<Socket> OutboundUpgrade<Socket> for RelayEvent
where
    Socket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = ();
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, mut socket: Socket, _: Self::Info) -> Self::Future {
        Box::pin(async move {
            trace!(
                "node_service/relay/upgrade_outbound: sending a new relay network event: {:?}",
                self
            );

            let bytes = serde_json::to_vec(&self).expect("failed to serialize RelayEvent to json");
            upgrade::write_one(&mut socket, bytes).await?;

            Ok(())
        })
    }
}
