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

use crate::connect_protocol::messages::{InMessage, OutMessage};
use libp2p::core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use log::trace;
use serde_json;
use std::{io, iter};
use tokio::prelude::*;

// 1 Mb
const MAX_BUF_SIZE: usize = 1 * 1024 * 1024;
const PROTOCOL_INFO: &[u8] = b"/janus/node/1.0.0";

impl UpgradeInfo for InMessage {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(PROTOCOL_INFO)
    }
}

impl<Socket: AsyncRead + AsyncWrite> InboundUpgrade<Socket> for InMessage {
    type Output = InMessage;
    type Error = failure::Error;
    type Future = upgrade::ReadOneThen<
        upgrade::Negotiated<Socket>,
        (),
        fn(Vec<u8>, ()) -> Result<Self::Output, Self::Error>,
    >;

    fn upgrade_inbound(
        self,
        socket: upgrade::Negotiated<Socket>,
        _info: Self::Info,
    ) -> Self::Future {
        upgrade::read_one_then(socket, MAX_BUF_SIZE, (), |packet, ()| {
            let relay_message: InMessage = serde_json::from_slice(&packet).unwrap();
            trace!("client: received a new relay message {:?}", relay_message);

            Ok(relay_message)
        })
    }
}

impl UpgradeInfo for OutMessage {
    type Info = &'static [u8];
    type InfoIter = iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        iter::once(PROTOCOL_INFO)
    }
}

impl<Socket> OutboundUpgrade<Socket> for OutMessage
where
    Socket: AsyncRead + AsyncWrite,
{
    type Output = ();
    type Error = io::Error;
    type Future = upgrade::WriteOne<upgrade::Negotiated<Socket>>;

    fn upgrade_outbound(
        self,
        socket: upgrade::Negotiated<Socket>,
        _info: Self::Info,
    ) -> Self::Future {
        trace!("client: sending a new network message: {:?}", self);

        let bytes = serde_json::to_vec(&self).expect("failed to serialize OutNodeMessage to json");
        upgrade::write_one(socket, bytes)
    }
}
