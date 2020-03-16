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

use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};

use itertools::Itertools;
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use parity_multihash::Multihash;

use crate::connect_protocol::events::ToPeerEvent;
use std::error::Error;

/// Describes message received by client from relay node; also see `ToPeerEvent`
#[derive(Debug, Clone)]
pub enum Message {
    Incoming {
        src: PeerId,
        data: String,
    },
    Providers {
        key: Multihash,
        providers: Vec<(Multiaddr, PeerId)>,
    },
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Message::Incoming { src, data } => write!(f, "{} from {}", data, src.to_base58()),
            Message::Providers { key, providers } => write!(
                f,
                "{} provided by {}",
                bs58::encode(key).into_string(),
                providers
                    .iter()
                    .map(|(addr, id)| format!("{}@{}", id.to_base58(), addr))
                    .join(", ")
            ),
        }
    }
}

#[derive(Debug, Clone)]
struct PeerIdErr<'a>(&'a str);
impl Error for PeerIdErr<'_> {}
impl Display for PeerIdErr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<ToPeerEvent> for Option<Message> {
    type Error = Box<dyn Error>;

    fn try_from(event: ToPeerEvent) -> Result<Self, Self::Error> {
        match event {
            ToPeerEvent::Deliver { src_id, data } => {
                let src = PeerId::from_bytes(src_id)
                    .map_err(|_| PeerIdErr("Error parsing src_id to PeerId"))?;
                let data = String::from_utf8(data)?;
                Ok(Some(Message::Incoming { src, data }))
            }
            ToPeerEvent::Providers { key, providers, .. } => {
                let key = Multihash::from_bytes(key)?;
                let providers = providers
                    .into_iter()
                    .map(|(addr, id)| PeerId::from_bytes(id).map(|id| (addr, id)))
                    .collect::<Result<_, _>>()
                    .map_err(|_| PeerIdErr("Error parsing providers to PeerId"))?;
                Ok(Some(Message::Providers { key, providers }))
            }
            ToPeerEvent::Upgrade => Ok(None),
        }
    }
}
