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

use faas_api::FunctionCall;
use janus_libp2p::peerid_serializer;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

/// Describes commands sent from client to relay node; also see `ToNodeNetworkMsg`
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", content = "body")]
pub enum Command {
    Call {
        #[serde(with = "peerid_serializer")]
        node: PeerId,
        call: FunctionCall,
    },
}
