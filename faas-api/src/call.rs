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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub sender: Address,
    /// List of modules to load when executing function
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<String>,
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
            sender: Default::default(),
            context: vec![],
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
            module: None,
            fname: None,
            arguments,
            name: name.into(),
            sender,
            context: vec![],
        }
    }
}

fn empty_obj() -> serde_json::Value {
    serde_json::Value::Object(<_>::default())
}

pub mod call_test_utils {
    use crate::FunctionCall;
    use crate::{provider, Address};

    pub fn gen_ipfs_call() -> FunctionCall {
        let sender = Address::random_relay();
        let reply_to = Some(sender.clone());
        let module = "IPFS.get_QmFile".to_string();
        let target = Some(provider!(module.clone()));

        FunctionCall {
            uuid: "UUID-1".to_string(),
            target,
            reply_to,
            module: Some(module.clone()),
            name: Some("Getting IPFS file QmFile".to_string()),
            sender,
            context: vec![module],
            ..<_>::default()
        }
    }

    pub fn gen_strict_ipfs_call() -> FunctionCall {
        let mut call = gen_ipfs_call();
        call.target = Some(Address::random_relay_unsigned());
        call.name = Some("Getting IPFS file QmFile (strict)".into());
        call
    }

    pub fn gen_provide_call(target: Address, arguments: serde_json::Value) -> FunctionCall {
        let sender = Address::random_relay();
        let reply_to = Some(sender.clone());

        FunctionCall {
            uuid: "UUID-1".to_string(),
            target: Some(target),
            reply_to,
            module: Some("provide".into()),
            arguments,
            sender,
            context: vec![],
            ..<_>::default()
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::call_test_utils::gen_strict_ipfs_call;
    use super::call_test_utils::{gen_ipfs_call, gen_provide_call};
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
    fn serialize_function() {
        let ipfs_call = gen_ipfs_call();
        check_call(ipfs_call);
        let strict_ipfs_call = gen_strict_ipfs_call();
        check_call(strict_ipfs_call);
    }

    #[test]
    fn serialize_provide() {
        let target = Protocol::Peer(RandomPeerId::random()).into();
        let arguments = json!({ "service_id": "IPFS.get_QmFile" });
        let mut call = gen_provide_call(target, arguments);
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
