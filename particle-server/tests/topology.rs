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

use serde_json::json;
use std::thread::sleep;
use test_utils::{make_swarms_with_cfg, ConnectedClient, KAD_TIMEOUT};

#[test]
fn identity() {
    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut a = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let mut b = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect client");

    a.send_particle(
        format!(
            r#"
                (seq (
                    (call ("{}" ("identity" "") () void0))
                    (call ("{}" ("identity" "") () void1))
                ))
            "#,
            b.node, b.peer_id
        ),
        json!({}),
    );

    b.receive_particle();
}
