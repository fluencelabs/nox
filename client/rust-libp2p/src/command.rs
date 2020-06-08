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

use faas_api::FunctionCall;
use serde::{Deserialize, Serialize};

/// Describes commands sent from client to relay node; also see `ToNodeNetworkMsg`
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum ClientCommand {
    Call { call: FunctionCall },
}

impl Into<FunctionCall> for ClientCommand {
    fn into(self) -> FunctionCall {
        match self {
            ClientCommand::Call { call } => call,
        }
    }
}
