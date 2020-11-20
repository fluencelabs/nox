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

#[macro_use]
extern crate fstrings;

use test_utils::{create_greeting_service, enable_logs, ConnectedClient};

use fstrings::f;
use maplit::hashmap;
use serde_json::json;

#[test]
fn create_service() {
    enable_logs();

    let service = create_greeting_service(3);

    let mut client =
        ConnectedClient::connect_to(service.other_hosts[0].1.clone()).expect("connect client");

    let script = f!(r#"
        (seq
            (seq
                (call relay ("identity" "") [])
                (call host (service_id "greeting") [my_name] greeting)
            )
            (seq
                (call relay ("identity" "") [])
                (call client ("return" "") [greeting])
            )
        )"#);
    client.send_particle(
        script,
        hashmap! {
            "host" => json!(service.host.0.to_string()),
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "service_id" => json!(service.service_id),
            "my_name" => json!("folex"),
        },
    );

    let response = client.receive_args();
    assert_eq!(response[0].as_str().unwrap(), "Hi, folex")
}
