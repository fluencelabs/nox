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

use test_utils::{make_swarms_with_cfg, ConnectedClient, KAD_TIMEOUT};

use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;
use std::thread::sleep;

#[test]
fn identity() {
    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut a = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();
    let mut b = ConnectedClient::connect_to(swarms[1].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    a.send_particle(
        r#"
        (seq
            (call node_a ("op" "identity") [])
            (seq
                (call node_b ("op" "identity") [])
                (seq
                    (call node_c ("op" "identity") [])
                    (seq
                        (call node_b ("op" "identity") [])
                        (call client_b ("op" "identity") [])
                    )
                )
            )
        )
        "#,
        hashmap! {
            "node_a" => json!(swarms[0].peer_id.to_string()),
            "node_b" => json!(swarms[1].peer_id.to_string()),
            "node_c" => json!(swarms[2].peer_id.to_string()),
            "client_b" => json!(b.peer_id.to_string()),
        },
    );

    b.receive().wrap_err("receive").unwrap();
}

#[test]
fn init_peer_id() {
    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
        (seq
            (call relay ("kad" "neighborhood") [client] peers)
            (seq
                (call relay ("op" "identity") [])
                (call %init_peer_id% ("event" "peers_discovered") [relay peers])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    client.receive().wrap_err("receive").unwrap();
}
