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

use crate::misc::peerid_serializer;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FunctionCall {
    pub uuid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Address>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<Address>,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub arguments: serde_json::Value, //TODO: make it Option<String>?
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl FunctionCall {
    pub fn reply(uuid: String, arguments: serde_json::Value) -> Self {
        FunctionCall {
            uuid,
            arguments,
            target: None,
            reply_to: None,
            name: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Address {
    Relay {
        #[serde(with = "peerid_serializer")]
        relay: PeerId,
        #[serde(with = "peerid_serializer")]
        client: PeerId,
    },
    Service {
        service: String,
    },
    Peer {
        #[serde(with = "peerid_serializer")]
        peer: PeerId,
    },
}

#[cfg(test)]
pub mod test {
    use crate::node_service::function::call::Address;
    use crate::node_service::function::FunctionCall;
    use libp2p::PeerId;
    use serde_json;

    #[test]
    fn serialize_address() {
        let p1 = PeerId::random();
        let p2 = PeerId::random();

        fn check(addr: Address) {
            let str = serde_json::to_string(&addr).unwrap();
            println!("{}", str);
            let addr_de: Address = serde_json::from_str(str.as_str()).unwrap();
            assert_eq!(addr, addr_de);
        }

        check(Address::Relay {
            relay: p1.clone(),
            client: p2,
        });

        check(Address::Service {
            service: "IPFS.get".to_string(),
        });

        check(Address::Peer { peer: p1 });
    }

    fn check_call(call: FunctionCall) {
        let str = serde_json::to_string(&call).unwrap();
        println!("{}", str);
        let call_de: FunctionCall = serde_json::from_str(str.as_str()).unwrap();

        assert_eq!(call, call_de);
    }

    pub fn gen_function_call() -> FunctionCall {
        let p1 = PeerId::random();
        let p2 = PeerId::random();
        let target = Some(Address::Relay {
            relay: p1,
            client: p2,
        });

        let reply_to = Some(Address::Service {
            service: "TelegramBot".to_string(),
        });

        let mut arguments = serde_json::Map::new();
        arguments.insert(
            "hash".to_string(),
            serde_json::Value::String("QmFile".to_string()),
        );
        let arguments = serde_json::Value::Object(arguments);

        FunctionCall {
            uuid: "UUID-1".to_string(),
            target,
            reply_to,
            arguments,
            name: Some("say_something_im_giving_up_on_you".to_string()),
        }
    }

    #[test]
    fn serialize_function() {
        let call = gen_function_call();
        check_call(call);
    }

    #[test]
    fn serialize_provide() {
        /*
        {
            "uuid": "UUID-1",
            "target": {
                "type": "Service",
                "service": "provide",
            },
            "reply_to": {
                "type": "Peer",
                "peer": "QmClient"
            },
            "arguments": {
                "key": "QmMyServiceId"
            },
        }
        */

        let p1 = PeerId::random();
        let p2 = PeerId::random();
        let mut arguments = serde_json::Map::new();
        arguments.insert(
            "key".to_string(),
            serde_json::Value::String("QmMyServiceId".to_string()),
        );
        let arguments = serde_json::Value::Object(arguments);
        let call = FunctionCall {
            uuid: "UUID-1".to_string(),
            target: Some(Address::Peer { peer: p1 }),
            reply_to: Some(Address::Peer { peer: p2 }),
            arguments,
            name: None,
        };
        check_call(call);
    }
}
