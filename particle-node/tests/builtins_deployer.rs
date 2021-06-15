/*
 * Copyright 2021 Fluence Labs Limited
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

use test_utils::{make_swarms_with_cfg, ConnectedClient};

use eyre::WrapErr;
use maplit::hashmap;
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

#[test]
fn builtins_test() {
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.builtins_dir = Some(Path::new("../deploy/builtins").into());
        cfg
    });

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"(xor
            (seq
                (seq
                    (call relay ("srv" "resolve_alias") [alias] service_id)
                    (seq
                        (call relay ("peer" "timestamp_sec") [] timestamp)
                        (call relay (service_id "register_key") [key timestamp pin weight] result)
                    )
                )
                (call %init_peer_id% ("op" "return") [result])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "alias" => json!("aqua-dht"),
            "key" => json!("some_key"),
            "pin" => json!(false),
            "weight" => json!(1u32),
        },
    );

    #[derive(Deserialize)]
    pub struct DhtResult {
        pub success: bool,
        pub error: String,
    }

    let result = client.receive_args().wrap_err("receive args").unwrap();
    let result = result.into_iter().next().unwrap();
    let result: DhtResult = serde_json::from_value(result).unwrap();

    assert!(result.success);
}
