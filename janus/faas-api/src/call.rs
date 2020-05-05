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

use janus_libp2p::peerid_serializer;
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Address {
    Relay {
        #[serde(with = "peerid_serializer")]
        // Provider of relay service
        relay: PeerId,
        #[serde(with = "peerid_serializer")]
        // Client of this relay
        client: PeerId,
    },
    Service {
        service_id: String,
    },
    Peer {
        #[serde(with = "peerid_serializer")]
        peer: PeerId,
    },
}

impl Address {
    pub fn destination_peer(&self) -> Option<&PeerId> {
        match self {
            Address::Relay { client, .. } => Some(client),
            Address::Peer { peer } => Some(peer),
            _ => None,
        }
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::Relay { client, relay } => write!(
                f,
                "Address [Peer {} via relay {}]",
                client.to_base58(),
                relay.to_base58()
            ),
            Address::Service { service_id } => write!(f, "Address [Service {}]", service_id),
            Address::Peer { peer } => write!(f, "Address [Peer {}]", peer),
        }
    }
}

pub mod call_test_utils {
    use super::Address;
    use super::FunctionCall;
    use libp2p::PeerId;

    pub fn gen_function_call() -> FunctionCall {
        let p1 = PeerId::random();
        let p2 = PeerId::random();
        let reply_to = Some(Address::Relay {
            relay: p1,
            client: p2,
        });

        let target = Some(Address::Service {
            service_id: "IPFS.get_QmFile".to_string(),
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
            name: Some("Getting IPFS file QmFile".to_string()),
        }
    }

    pub fn gen_provide_call(target: Address, arguments: serde_json::Value) -> FunctionCall {
        let notebook = PeerId::random();
        let relay = PeerId::random();
        let reply_to = Some(Address::Relay {
            client: notebook,
            relay,
        });

        FunctionCall {
            uuid: "UUID-1".to_string(),
            target: Some(target),
            reply_to,
            arguments,
            name: None,
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::call_test_utils::{gen_function_call, gen_provide_call};
    use super::Address;
    use super::FunctionCall;
    use libp2p::PeerId;
    use serde_json::json;

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
            service_id: "IPFS.get".to_string(),
        });

        check(Address::Peer { peer: p1 });
    }

    fn check_call(call: FunctionCall) {
        let str = serde_json::to_string(&call).unwrap();
        println!("{}", str);
        let call_de: FunctionCall = serde_json::from_str(str.as_str()).unwrap();

        assert_eq!(call, call_de);
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
                "service_id": "provide",
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

        let service = Address::Service {
            service_id: "provide".into(),
        };
        let arguments = json!({ "service_id": "IPFS.get_QmFile" });
        let call = gen_provide_call(service, arguments);

        check_call(call.clone());

        match call.target.clone() {
            Some(Address::Service { service_id }) if service_id.as_str() == "provide" => {}
            wrong => panic!("target should be Some(Address::Service), was {:?}", wrong),
        };
    }

    #[test]
    fn serialize_reply_to_service() {
        use serde_json::json;

        let slack_service = "hash(Slack.receiveWebhook_0xdxSECRET_CODE)";
        let github_service = "Github.subscribeNewCommitsToWebhook";

        let slack_service = Some(Address::Service {
            service_id: slack_service.into(),
        });
        let github_service = Some(Address::Service {
            service_id: github_service.into(),
        });
        let arguments = json!({"repo": "fluencelabs/fluence", "branch": "all"});

        // Notebook sends a call to github, and now github will send new events to slack
        let call = FunctionCall {
            uuid: "UUID-1".to_string(),
            target: github_service,
            reply_to: slack_service,
            arguments,
            name: None,
        };
        check_call(call.clone());
    }
}
