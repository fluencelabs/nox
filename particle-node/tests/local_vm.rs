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

#[allow(unused_imports)]
#[macro_use]
extern crate fstrings;

use std::time::Duration;

use maplit::hashmap;
use serde_json::json;

use fluence_libp2p::RandomPeerId;
use local_vm::{make_particle, make_vm, read_args};
use log_utils::enable_logs;

#[test]
fn make() {
    enable_logs();

    let client_a = RandomPeerId::random();
    let client_b = RandomPeerId::random();

    let mut local_vm_a = make_vm(client_a);
    let mut local_vm_b = make_vm(client_b);

    let script = r#"(call client_b ("return" "") [a b c])"#.to_string();
    let data = hashmap! {
        "client_b" => json!(client_b.to_string()),
        "a" => json!("a_value"),
        "b" => json!(["b1", "b2", "b3"]),
        "c" => json!({"c1": "c1_value", "c2": "c2_value"})
    };

    let data = data
        .iter()
        .map(|(key, value)| (key.to_string(), value.clone()))
        .collect();

    let particle = make_particle(
        client_a,
        &data,
        script,
        None,
        &mut local_vm_a,
        false,
        Duration::from_secs(20),
    );

    let args = read_args(particle, client_b, &mut local_vm_b)
        .expect("read args")
        .expect("read args");
    assert_eq!(data["a"], args[0]);
    assert_eq!(data["b"], args[1]);
    assert_eq!(data["c"], args[2]);
}
