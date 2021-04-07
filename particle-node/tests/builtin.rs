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

use test_utils::{make_swarms, ConnectedClient};

use eyre::WrapErr;
use libp2p::core::Multiaddr;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Debug)]
struct NodeInfo {
    pub external_addresses: Vec<Multiaddr>,
}

#[test]
fn identify() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
        (seq
            (call relay ("peer" "identify") [] info)
            (call client ("op" "return") [info])
        ) 
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let info = client.receive_args().wrap_err("receive args").unwrap();
    let info = info.into_iter().next().unwrap();
    let _: NodeInfo = serde_json::from_value(info).unwrap();
}

#[test]
fn deserialize() {
    let msg = FooMessage::Particle(<_>::default());
    let bytes = serde_json::to_vec(&msg).unwrap();
    let test_msg: Result<FooMessage, _> = serde_json::from_slice(&bytes);
    println!("{:?}", test_msg);
    test_msg.unwrap();
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct FooParticle {
    pub id: String,
    // #[serde(with = "peerid_serializer")]
    // pub init_peer_id: PeerId,
    pub timestamp: u64,
    pub ttl: u32,
    pub script: String,
    pub signature: Vec<u8>,
    // base64-encoded
    // #[serde(with = "base64_serde")]
    // #[derivative(Debug(format_with = "fmt_data"))]
    // pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "action")]
pub enum FooMessage {
    Particle(FooParticle),
    // InboundUpgradeError(serde_json::Value),
    // Upgrade,
}
