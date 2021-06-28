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

use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;

use log_utils::enable_logs;
use services_utils::load_module;
use test_utils::{create_service, make_swarms, ConnectedClient, CreatedService};

fn create_file_share(client: &mut ConnectedClient) -> CreatedService {
    create_service(
        client,
        "file_share",
        load_module("tests/file_share/artifacts", "file_share"),
    )
}

#[test]
fn share_file() {
    enable_logs();

    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let first = create_file_share(&mut client);
    let second = create_file_share(&mut client);

    client.send_particle(
        r#"
        (xor
            (seq
                (seq
                    (call relay (first "create_vault_file") [input_content] filename)
                    (call relay (second "read_vault_file" ) [filename] output_content)
                )
                (call %init_peer_id% ("op" "return") [output_content])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.msg])
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
        assert_eq!(output, "Hello");
    } else {
        panic!("incorrect args: expected a single string")
    }
}
