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

use test_utils::{
    enable_logs, make_swarms, make_swarms_with_transport, ConnectedClient, Transport,
};

use eyre::WrapErr;
use libp2p::core::Multiaddr;
use maplit::hashmap;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

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
fn big_identity() {
    enable_logs();

    let swarms = make_swarms_with_transport(1, Transport::Network);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    // let mut client = ConnectedClient::connect_to("/ip4/157.230.98.75/tcp/7001".parse().unwrap())
    //     .wrap_err("connect client")
    //     .unwrap();

    client.send_particle(
        r#"
        (seq
            (call relay ("op" "identity") [data] result)
            (call %init_peer_id% ("op" "return") [result])
        ) 
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "data" => json!(base64::encode((0..(1024*1024*20)).map(|_| u8::MAX).collect::<Vec<_>>())),
        },
    );

    client.timeout = Duration::from_secs(60);
    client.receive_args().wrap_err("receive args").unwrap();
}
