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

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;

use connected_client::ConnectedClient;
use created_swarm::make_swarms;
use service_modules::load_module;
use test_utils::{create_service, CreatedService};

fn create_file_share(client: &mut ConnectedClient) -> CreatedService {
    create_service(
        client,
        "file_share",
        load_module("tests/file_share/artifacts", "file_share").expect("load module"),
    )
}

#[test]
fn share_file() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let first = create_file_share(&mut client);
    let second = create_file_share(&mut client);

    client.send_particle(
        r#"
        (seq
            (call relay ("srv" "get_interface") [first] interface)
            (xor
                (seq
                    (seq
                        (call relay (first "create_vault_file") [input_content] filename)
                        (call relay (second "read_vault_file" ) [filename] output_content)
                    )
                    (call %init_peer_id% ("op" "return") [output_content])
                )
                (call %init_peer_id% ("op" "return") [%last_error%.$.message interface])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "first" => json!(first.id),
            "second" => json!(second.id),
            "input_content" => json!("Hello!")
        },
    );

    use serde_json::Value::String;

    if let [String(output)] = client.receive_args().unwrap().as_slice() {
        assert_eq!(output, "Hello!");
    } else {
        panic!("incorrect args: expected a single string")
    }
}

#[test]
fn deploy_from_vault() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let file_share = create_file_share(&mut client);
    let module = load_module("tests/file_share/artifacts", "file_share").expect("load module");

    client.send_particle(
        r#"
        (seq
            (seq
                (seq
                    (call relay (first_service "create_base64_vault_file") [module] filename)
                    (seq
                        (call relay ("op" "concat_strings") ["{" q "name" q ": " q "file_share" q "}"] config_string)
                        (call relay (first_service "create_vault_file") [config_string] config_filename)
                    )
                )
                (seq 
                    (call relay ("dist" "load_module_config") [config_filename] module_config)
                    (seq
                        (call relay ("dist" "add_module_from_vault") [filename module_config] module_hash)
                        (seq
                            (call relay ("op" "concat_strings") ["hash:" module_hash] annotated_hash)
                            (seq
                                (call relay ("op" "array") [annotated_hash] dependencies)
                                (seq
                                    (call relay ("dist" "make_blueprint") ["file_share" dependencies] blueprint)
                                    (seq
                                        (call relay ("dist" "add_blueprint") [blueprint] blueprint_id)
                                        (seq
                                            (call relay ("srv" "create") [blueprint_id] second_service)
                                            (call relay (second_service "read_base64_vault_file") [filename] output_content)
                                        )
                                    )
                                )
                            )
                        )                
                    )
                )
            )
            (call %init_peer_id% ("op" "return") [output_content])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "first_service" => json!(file_share.id),
            "module" => json!(base64.encode(&module)),
            "q" => json!("\""),
        },
    );

    use serde_json::Value::String;

    let args = client.receive_args().unwrap();
    if let [String(output)] = args.as_slice() {
        assert_eq!(base64.decode(output).unwrap(), module);
    } else {
        panic!("#incorrect args: expected a single string, got {:?}", args);
    }
}

#[test]
fn load_blueprint_from_vault() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    // upload module
    let module = load_module("tests/file_share/artifacts", "file_share").expect("load module");
    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("dist" "default_module_config") ["file_share"] config)                
                (call relay ("dist" "add_module") [module config] hash)
            )
            (call %init_peer_id% ("op" "return") [hash])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "module" => json!(base64.encode(module)),
        },
    );

    let args = client.receive_args().unwrap();
    let module_hash = args[0].as_str().expect("single string");

    // create service from blueprint stored in vault
    let file_share = create_file_share(&mut client);

    let blueprint_string = json!({
        "name": "file_share",
        "dependencies": [format!("hash:{module_hash}")]
    })
    .to_string();
    client.send_particle(
        r#"
        (seq
            (seq
                (call relay (first_service "create_vault_file") [blueprint_string] filename)
                (seq
                    (call relay ("dist" "load_blueprint") [filename] blueprint)
                    (seq
                        (call relay ("dist" "add_blueprint") [blueprint] blueprint_id)
                        (call relay ("srv" "create") [blueprint_id] second_service)
                    )
                )
            )
            (seq
                (call relay (second_service "read_vault_file") [filename] output_content)
                (call %init_peer_id% ("op" "return") [output_content])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "first_service" => json!(file_share.id),
            "blueprint_string" => json!(blueprint_string),
        },
    );

    use serde_json::Value::String;

    let args = client.receive_args().unwrap();
    if let [String(output)] = args.as_slice() {
        assert_eq!(output, &blueprint_string);
    } else {
        panic!("#incorrect args: expected a single string, got {:?}", args);
    }
}
