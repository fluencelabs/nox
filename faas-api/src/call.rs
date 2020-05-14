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
    pub fn with_target(mut self, target: Address) -> Self {
        self.target = Some(target);
        self
    }
}

pub mod call_test_utils {
    use crate::Address;
    use crate::FunctionCall;
    use crate::Protocol;
    use libp2p::PeerId;

    pub fn gen_ipfs_call() -> FunctionCall {
        let relay = PeerId::random();
        let client = PeerId::random();
        #[rustfmt::skip]
        let reply_to: Option<Address> = Some(
            vec![Protocol::Peer(relay), Protocol::Client(client)].iter().collect(),
        );

        let target = Some(Protocol::Service("IPFS.get_QmFile".to_string()).into());

        FunctionCall {
            uuid: "UUID-1".to_string(),
            target,
            reply_to,
            arguments: serde_json::Value::Null,
            name: Some("Getting IPFS file QmFile".to_string()),
        }
    }

    pub fn gen_strict_ipfs_call() -> FunctionCall {
        let mut call = gen_ipfs_call();
        let relay: Address = Protocol::Peer(PeerId::random()).into();
        let notebook = relay.append(Protocol::Client(PeerId::random()));
        let service_on_notebook = notebook.extend(&call.target.unwrap());
        call.target = Some(service_on_notebook);
        call.name = Some("Getting IPFS file QmFile (strict)".into());
        call
    }

    pub fn gen_provide_call(target: Address, arguments: serde_json::Value) -> FunctionCall {
        let notebook = Protocol::Client(PeerId::random());
        let relay = Protocol::Peer(PeerId::random());
        let reply_to = Some(relay / notebook);

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
    use super::call_test_utils::gen_strict_ipfs_call;
    use super::call_test_utils::{gen_ipfs_call, gen_provide_call};
    use crate::Address;
    use crate::FunctionCall;
    use crate::Protocol;
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

        check(Address::from(Protocol::Peer(p1.clone())).append(Protocol::Client(p2)));
        check(Protocol::Service("IPFS.get".to_string()).into());
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
        use Protocol::*;

        let provide: Address = Service("provide".into()).into();
        let provide = provide.append(Service("IPFS.get_QmFile".into()));
        let mut call = gen_provide_call(provide, serde_json::Value::Null);
        call.name = Some("Announce IPFS file (in address)".into());

        check_call(call.clone());

        assert_eq!(
            call.target,
            Some("/service/provide/service/IPFS.get_QmFile".parse().unwrap())
        );

        let provide = Service("provide".into()).into();
        let arguments = json!({ "service_id": "IPFS.get_QmFile" });
        let mut call = gen_provide_call(provide, arguments);
        call.name = Some("Announce IPFS file (in args)".into());
        check_call(call.clone());

        assert_eq!(call.target, Some("/service/provide".parse().unwrap()));

        let provide = Service("provide".into()).into();
        let mut call = gen_provide_call(provide, serde_json::Value::Null);
        let notebook = call.reply_to.take().unwrap();
        call.reply_to = Some(notebook.append(Protocol::Service("IPFS.get_QmFile".into())));
        call.name = Some("Announce IPFS file (in reply)".into());
        check_call(call.clone());

        assert_eq!(call.target, Some("/service/provide".parse().unwrap()));
    }

    #[test]
    fn serialize_reply_to_service() {
        use serde_json::json;
        use Protocol::*;

        let slack_service = "hash(Slack.receiveWebhook_0xdxSECRET_CODE)";
        let github_service = "Github.subscribeNewCommitsToWebhook";

        let slack_service = Some(Service(slack_service.into()).into());
        let github_service = Some(Service(github_service.into()).into());
        let arguments = json!({"repo": "fluencelabs/fluence", "branch": "all"});

        // Notebook sends a call to github, and now github will send new events to slack
        let call = FunctionCall {
            uuid: "UUID-1".to_string(),
            target: github_service,
            reply_to: slack_service,
            arguments,
            name: Some("Subscribing Slack channel to Github commits".into()),
        };
        check_call(call);
    }
}
