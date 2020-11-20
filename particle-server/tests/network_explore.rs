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
    create_greeting_service, enable_logs, make_swarms, make_swarms_with_cfg, ConnectedClient,
    KAD_TIMEOUT,
};

use maplit::hashmap;
use serde_json::json;
use std::thread::sleep;

#[test]
fn get_active_interfaces() {
    enable_logs();

    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let service1 = create_greeting_service(&mut client);
    let service2 = create_greeting_service(&mut client);

    client.send_particle(
        r#"
        (seq
            (call relay ("get_active_interfaces" "") [] interfaces)
            (call client ("return" "") [interfaces])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let args = client.receive_args();
    println!("{}", serde_json::Value::Array(args));
}
