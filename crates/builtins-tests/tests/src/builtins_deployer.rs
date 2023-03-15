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

use std::time::Duration;
use std::{env, fs, path::Path};

use eyre::WrapErr;
use fluence_keypair::KeyPair;
use fstrings::f;
use maplit::hashmap;
use serde::Deserialize;
use serde_json::json;

use builtins_deployer::ALLOWED_ENV_PREFIX;
use connected_client::ConnectedClient;
use created_swarm::{make_swarms_with_builtins, make_swarms_with_keypair};
use fs_utils::copy_dir_all;
use fs_utils::list_files;
use service_modules::load_module;
use test_utils::create_service;

use crate::{SERVICES, SPELL};

async fn check_registry_builtin(client: &mut ConnectedClient) {
    // TODO: get rid of FIVE SECONDS sleep
    tokio::time::sleep(Duration::from_millis(5000)).await;

    let mut result = client
        .execute_particle(
            r#"(xor
            (seq
                (seq
                    (call relay ("srv" "resolve_alias") [alias] service_id)
                    (seq
                        (call relay ("peer" "timestamp_sec") [] timestamp)
                        (call relay (service_id "get_key_id") [label %init_peer_id%] result)
                    )
                )
                (call %init_peer_id% ("op" "return") [result])
            )
            (call %init_peer_id% ("op" "return") [%last_error%])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "alias" => json!("registry"),
                "label" => json!("some_label"),
            },
        )
        .await
        .unwrap();
    match result.pop() {
        Some(serde_json::Value::String(s)) => assert!(s.contains("some_label")),
        other => panic!("expected json string, got {:?}", other),
    }
}

#[tokio::test]
async fn builtins_test() {
    let swarms =
        make_swarms_with_builtins(1, Path::new(SERVICES), None, Some(SPELL.to_string())).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    check_registry_builtin(&mut client).await;
}

#[tokio::test]
async fn builtins_replace_old() {
    let keypair = KeyPair::generate_ed25519();
    let swarms = make_swarms_with_keypair(1, keypair.clone(), Some(SPELL.to_string())).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    // use tetraplets as aqua-dht to emulate old builtin being replaced by a new version
    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module(
            "../particle-node-tests/tests/tetraplets/artifacts",
            "tetraplets",
        )
        .expect("load module"),
    )
    .await;

    let result = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    let result = result[0].as_str().unwrap();
    assert_eq!(result, "ok");

    // stop swarm
    swarms.into_iter().map(|s| s.outlet.send(())).for_each(drop);

    // restart with same keypair
    let swarms = make_swarms_with_builtins(
        1,
        Path::new(SERVICES),
        Some(keypair),
        Some(SPELL.to_string()),
    )
    .await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    check_registry_builtin(&mut client).await;
}

#[tokio::test]
async fn builtins_scheduled_scripts() {
    let swarms =
        make_swarms_with_builtins(1, Path::new(SERVICES), None, Some(SPELL.to_string())).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let result = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    let result = result[0].as_array().unwrap();

    let mut scripts_count = 0;
    for dir in list_files(Path::new(SERVICES)).unwrap() {
        scripts_count += list_files(&dir.join("scheduled")).unwrap().count();
    }
    assert_eq!(result.len(), scripts_count)
}

#[tokio::test]
#[ignore]
async fn builtins_resolving_env_variables() {
    copy_dir_all(SERVICES, "./builtins_test_env").unwrap();
    let key = "some_key".to_string();
    let on_start_script = f!(r#"
    (xor
        (seq
            (seq
                (call relay ("peer" "timestamp_sec") [] timestamp0)
                (call relay ("aqua-dht" "register_key") [key timestamp0 false 0])
            )
            (call relay ("op" "return") ["ok"])
        )
        (call relay ("op" "return") [%last_error%.$.instruction])
    )
    "#);
    let env_variable_name = format!("{}_AQUA_DHT_{}", ALLOWED_ENV_PREFIX, "KEY");
    let on_start_data = json!({ "key": env_variable_name });
    env::set_var(&env_variable_name[1..], key.clone());
    fs::write("./builtins_test_env/aqua-dht/on_start.air", on_start_script).unwrap();
    fs::write(
        "./builtins_test_env/aqua-dht/on_start.json",
        on_start_data.to_string(),
    )
    .unwrap();

    let swarms = make_swarms_with_builtins(
        1,
        Path::new("./builtins_test_env"),
        None,
        Some(SPELL.to_string()),
    )
    .await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let result = client
        .execute_particle(
            f!(r#"(xor
            (seq
                (seq
                    (call relay ("peer" "timestamp_sec") [] timestamp1)
                    (call relay ("aqua-dht" "get_key_metadata") ["{key}" timestamp1] result)
                )
                (call %init_peer_id% ("op" "return") [result])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#),
            hashmap! {
                "relay" => json!(client.node.to_string()),
            },
        )
        .await
        .unwrap();

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
    fs::remove_dir_all("./builtins_test_env").unwrap();
}
