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

use libp2p::PeerId;
use parity_multihash::Multihash;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::error::Error;

/// Describes commands sent from client to relay node; also see `ToNodeEvent`
#[derive(Debug, Clone)]
pub enum Command {
    Relay { dst: PeerId, data: String },
    Provide(Multihash),
    FindProviders(Multihash),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum TextCommand {
    Relay { dst: String, message: String },
    Provide { key: String },
    FindProviders { key: String },
}

impl TryFrom<TextCommand> for Command {
    type Error = Box<dyn Error>;

    fn try_from(cmd: TextCommand) -> Result<Self, Self::Error> {
        match cmd {
            TextCommand::Relay { dst, message } => Ok(Command::Relay {
                dst: dst.parse()?,
                data: message,
            }),
            TextCommand::Provide { key } => {
                Ok(Command::Provide(bs58::decode(key).into_vec()?.try_into()?))
            }
            TextCommand::FindProviders { key } => Ok(Command::FindProviders(
                bs58::decode(key).into_vec()?.try_into()?,
            )),
        }
    }
}
