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

use crate::Particle;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "action")]
pub enum ProtocolMessage {
    Particle(Particle),
    UpgradeError(serde_json::Value),
    Upgrade,
}

impl Default for ProtocolMessage {
    fn default() -> Self {
        ProtocolMessage::Upgrade
    }
}

// required by OneShotHandler in inject_fully_negotiated_outbound
impl Into<ProtocolMessage> for () {
    fn into(self) -> ProtocolMessage {
        ProtocolMessage::Upgrade
    }
}
