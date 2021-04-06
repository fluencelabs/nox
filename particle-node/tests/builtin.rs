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
use serde::Deserialize;
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
    let bytes = vec![
        123, 34, 97, 99, 116, 105, 111, 110, 34, 58, 34, 80, 97, 114, 116, 105, 99, 108, 101, 34,
        44, 34, 105, 100, 34, 58, 34, 49, 34, 44, 34, 105, 110, 105, 116, 95, 112, 101, 101, 114,
        95, 105, 100, 34, 58, 34, 49, 50, 68, 51, 75, 111, 111, 87, 67, 74, 104, 76, 98, 78, 51,
        118, 67, 101, 112, 109, 70, 106, 114, 87, 70, 53, 90, 70, 68, 71, 65, 117, 65, 89, 86, 121,
        78, 74, 51, 70, 49, 49, 101, 80, 99, 119, 76, 76, 82, 120, 86, 76, 34, 44, 34, 116, 105,
        109, 101, 115, 116, 97, 109, 112, 34, 58, 49, 54, 49, 55, 55, 51, 55, 48, 49, 54, 57, 51,
        49, 44, 34, 116, 116, 108, 34, 58, 54, 53, 53, 50, 53, 44, 34, 115, 99, 114, 105, 112, 116,
        34, 58, 34, 34, 44, 34, 115, 105, 103, 110, 97, 116, 117, 114, 101, 34, 58, 91, 93, 44, 34,
        100, 97, 116, 97, 34, 58, 34, 34, 125,
    ];
    let test_msg: Result<particle_protocol::ProtocolMessage, _> = serde_json::from_slice(&bytes);
    println!("{:?}", test_msg);
    test_msg.unwrap();
}
