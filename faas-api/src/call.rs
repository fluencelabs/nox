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

use crate::Address;
use fluence_app_service::CallParameters;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FunctionCall {
    pub uuid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Address>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<Address>,
    pub module: Option<String>,
    pub fname: Option<String>,
    #[serde(
        default = "empty_obj",
        skip_serializing_if = "serde_json::Value::is_null"
    )]
    pub arguments: serde_json::Value, //TODO: make it Option<String>?
    #[serde(default)]
    pub headers: CallParameters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub sender: Address,
}

impl Default for FunctionCall {
    /// Implement default manually so arguments are `{}` by default
    fn default() -> Self {
        Self {
            uuid: "".to_string(),
            target: None,
            reply_to: None,
            module: None,
            fname: None,
            arguments: empty_obj(),
            name: None,
            sender: <_>::default(),
            headers: <_>::default(),
        }
    }
}

impl FunctionCall {
    /// Set target to a call, and return the call
    pub fn with_target(mut self, target: Address) -> Self {
        self.target = Some(target);
        self
    }

    /// Build a "reply" call from given arguments
    pub fn reply<O>(target: Address, sender: Address, arguments: serde_json::Value, name: O) -> Self
    where
        O: Into<Option<String>>,
    {
        Self {
            uuid: Uuid::new_v4().to_string(),
            target: Some(target),
            reply_to: Some(sender.clone()),
            arguments,
            name: name.into(),
            sender,
            ..<_>::default()
        }
    }

    pub fn msg_id(&self) -> Option<&'_ str> {
        self.arguments.get("msg_id").and_then(|s| s.as_str())
    }
}

fn empty_obj() -> serde_json::Value {
    serde_json::Value::Object(<_>::default())
}

#[cfg(test)]
pub mod test {
    use crate::Address;
    use crate::FunctionCall;
    use crate::Protocol;
    use crate::{hashtag, provider, relay};
    use fluence_libp2p::RandomPeerId;
    use serde_json::json;

    #[test]
    fn serialize_address() {
        let p1 = RandomPeerId::random();
        let p2 = RandomPeerId::random();

        fn check(addr: Address) {
            let str = serde_json::to_string(&addr).unwrap();
            println!("{}", str);
            let addr_de: Address = serde_json::from_str(str.as_str()).unwrap();
            assert_eq!(addr, addr_de);
        }

        check(relay!(p1.clone(), p2));
        check(provider!("IPFS.get"));
        check(Protocol::Peer(p1).into());
    }

    fn check_call(call: FunctionCall) {
        let str = serde_json::to_string(&call).unwrap();
        println!("{}", str);
        let call_de: FunctionCall = serde_json::from_str(str.as_str()).unwrap();

        assert_eq!(call, call_de);
    }

    #[test]
    fn serialize_provide() {
        let mut call = FunctionCall::random();
        call.module = Some("provide".to_string());
        call.arguments = json!({ "name": "IPFS", "address": Address::random_relay() });
        call.name = Some("Announce IPFS file (in args)".into());
        check_call(call.clone());

        assert_eq!(call.module, Some("provide".to_string()));
    }

    #[test]
    fn serialize_reply_from_service() {
        use serde_json::json;

        let slack_module = "hash(Slack.receiveWebhook_0xdxSECRET_CODE)".to_string();

        let slack_service = provider!(slack_module.clone());
        let github_node = Some(Address::random_relay() / hashtag!("msgId123"));
        let arguments = json!({"repo": "fluencelabs/fluence", "branch": "all"});

        // Notebook sends a call to github, and now github will send new events to slack
        let call = FunctionCall {
            uuid: "UUID-1".to_string(),
            target: Some(slack_service),
            reply_to: github_node,
            module: Some(slack_module),
            arguments,
            name: Some("Subscribing Slack channel to Github commits".into()),
            sender: Address::random_relay(),
            ..<_>::default()
        };
        check_call(call);
    }
}
