/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use crate::FunctionCall;
use serde::{Deserialize, Serialize};

/// `RawData` could be a byte array, string, `serde_json::Value`, or something like that
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "action")]
#[allow(clippy::large_enum_variant)]
pub enum ProtocolMessage {
    FunctionCall(FunctionCall),
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
    use crate::call_test_utils::gen_function_call;
    use crate::ProtocolMessage;

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

    #[test]
    fn deserialize() {
        let example1 = r#"
            {
                "uuid": "2020-04-09T11:45:54.000Z",
                "target": {
                    "service_id": "provide",
                    "type": "Service"
                },
                "reply_to": {
                    "relay": "QmX6yYZd4iLW7YpmZz4waLrtb5Y9f5v3PPGEmNGh9k3iW2",
                    "client": "QmcsjjDd8bHFXwAttvyhp7CgaysZhABE2tXFjfPLA5ABJ5",
                    "type": "Relay"
                },
                "arguments": {
                    "service_id": "println summa"
                },
                "name": "provide service",
                "action": "FunctionCall"
            }
            "#;

        let example2 = r#"
            {
                "uuid": "2020-04-09T11:47:42.937Z",
                "target": {
                    "service_id": "provide",
                    "type": "Service"
                },
                "reply_to": {
                    "relay": "QmX6yYZd4iLW7YpmZz4waLrtb5Y9f5v3PPGEmNGh9k3iW2",
                    "client": "QmcsjjDd8bHFXwAttvyhp7CgaysZhABE2tXFjfPLA5ABJ5",
                    "type": "Relay"
                },
                "arguments": {
                    "service_id": "println summa"
                },
                "name": "provide service",
                "action": "FunctionCall"
            }
        "#;

        let _msg: ProtocolMessage = serde_json::from_str(example1).unwrap();
        let _msg: ProtocolMessage = serde_json::from_str(example2).unwrap();
    }
}
