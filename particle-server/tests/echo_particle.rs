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

use test_utils::{make_swarms, ConnectedClient, KAD_TIMEOUT};

use serde_json::json;
use std::thread::sleep;

#[test]
fn echo_particle() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    client.send_particle(
        format!(
            r#"(call ("{}" ("service_id" "fn_name") () result_name))"#,
            client.peer_id
        ),
        json!({}),
    );
    client.receive();
}
