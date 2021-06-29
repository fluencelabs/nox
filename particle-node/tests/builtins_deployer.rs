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

#[macro_use]
extern crate fstrings;

use connected_client::ConnectedClient;
use created_swarm::{make_swarms_with_builtins, make_swarms_with_keypair};
use particle_modules::list_files;
use service_modules::load_module;
use test_utils::create_service;

use libp2p::core::identity::Keypair;

use eyre::WrapErr;
use maplit::hashmap;
use particle_node::ALLOWED_ENV_PREFIX;
use serde::Deserialize;
use serde_json::json;
use std::{env, fs, path::Path};

fn check_dht_builtin(client: &mut ConnectedClient) {
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

#[test]
fn builtins_test() {
    let swarms = make_swarms_with_builtins(1, Path::new("../deploy/builtins"), None);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    check_dht_builtin(&mut client);
}

#[test]
fn builtins_replace_old() {
    let keypair = Keypair::generate_ed25519();
    let swarms = make_swarms_with_keypair(1, keypair.clone());

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .wrap_err("connect client")
    .unwrap();

    // use this with aqua-dht alias to emulate old builtin
    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    );

    client.send_particle(
        r#"
        (xor
            (seq
                (call relay ("srv" "add_alias") [alias service])
                (call %init_peer_id% ("op" "return") ["ok"])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "service" => json!(tetraplets_service.id),
            "alias" => json!("aqua-dht".to_string()),
        },
    );

    let result = client.receive_args().unwrap();
    let result = result[0].as_str().unwrap();
    assert_eq!(result, "ok");

    // stop swarm
    swarms.into_iter().map(|s| s.outlet.send(())).for_each(drop);

    // restart with same keypair
    let swarms = make_swarms_with_builtins(1, Path::new("../deploy/builtins"), Some(keypair));

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    check_dht_builtin(&mut client);
}

#[test]
fn builtins_scheduled_scripts() {
    let swarms = make_swarms_with_builtins(1, Path::new("../deploy/builtins"), None);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"(xor
            (seq
                (call relay ("script" "list") [] result)
                (call %init_peer_id% ("op" "return") [result])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
        },
    );

    let result = client.receive_args().wrap_err("receive args").unwrap();
    let result = result[0].as_array().unwrap();
    assert_eq!(
        result.len(),
        list_files(Path::new("../deploy/builtins/aqua-dht/scheduled"))
            .unwrap()
            .count()
    )
}

#[test]
fn builtins_resolving_env_variables() {
    let key = "some_key".to_string();
    let on_start_script = f!(r#"
    (xor
        (seq
            (seq
                (call relay ("peer" "timestamp_sec") [] timestamp)
                (call relay ("aqua-dht" "register_key") [key timestamp false 0])
            )
            (call %init_peer_id% ("op" "return") ["ok"])
        )
        (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
    )
    "#);
    let env_variable_name = format!("{}_AQUA_DHT", ALLOWED_ENV_PREFIX);
    let on_start_data = json!({ "key": env_variable_name.clone() });
    env::set_var(&env_variable_name[1..], key.clone());
    fs::write("../deploy/builtins/aqua-dht/on_start.air", on_start_script).unwrap();
    fs::write(
        "../deploy/builtins/aqua-dht/on_start.json",
        on_start_data.to_string(),
    )
    .unwrap();

    let swarms = make_swarms_with_builtins(1, Path::new("../deploy/builtins"), None);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        f!(r#"(xor
            (seq
                (seq
                    (call relay ("peer" "timestamp_sec") [] timestamp)
                    (call relay ("aqua-dht" "get_key_metadata") ["{key}" timestamp] result)
                )
                (call %init_peer_id% ("op" "return") [result])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#),
        hashmap! {
            "relay" => json!(client.node.to_string()),
        },
    );

    let result = client.receive_args().wrap_err("receive args").unwrap();

    #[derive(Deserialize)]
    pub struct Key {
        pub key: String,
        pub peer_id: String,
        pub timestamp_created: u64,
        pub pinned: bool,
        pub weight: u32,
    }

    #[derive(Deserialize)]
    pub struct GetKeyMetadataResult {
        pub success: bool,
        pub error: String,
        pub key: Key,
    }

    let result = result.into_iter().next().unwrap();
    let result: GetKeyMetadataResult = serde_json::from_value(result).unwrap();

    assert!(result.success);
    assert_eq!(key, result.key.key);
    fs::remove_file("../deploy/builtins/aqua-dht/on_start.air").unwrap();
    fs::remove_file("../deploy/builtins/aqua-dht/on_start.json").unwrap();
}
