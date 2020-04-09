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

use crate::node_service::function::call;
use serde::{Deserialize, Serialize};

/// `RawData` could be a byte array, string, `serde_json::Value`, or something like that
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "action")]
pub enum ProtocolMessage {
    FunctionCall(call::FunctionCall),
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

#[cfg(test)]
mod test {
    use crate::node_service::function::{gen_function_call, ProtocolMessage};

    fn check_msg(msg: ProtocolMessage) {
        let str = serde_json::to_string(&msg).unwrap();
        println!("{}", str);
        let call_de: ProtocolMessage = serde_json::from_str(str.as_str()).unwrap();

        assert_eq!(msg, call_de);
    }

    #[test]
    fn protocol_message() {
        let msg = ProtocolMessage::FunctionCall(gen_function_call());
        check_msg(msg);
    }
}
