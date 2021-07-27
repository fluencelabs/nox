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

use fluence_libp2p::RandomPeerId;
use local_vm::{make_call_service_closure, make_particle, make_vm, read_args};

use maplit::hashmap;
use parking_lot::Mutex;
use serde_json::json;
use serde_json::Value as JValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn make() {
    let client_a = RandomPeerId::random();
    let client_b = RandomPeerId::random();

    let call_service_in_a: Arc<Mutex<HashMap<String, JValue>>> = <_>::default();
    let call_service_out_a: Arc<Mutex<Vec<JValue>>> = <_>::default();
    let mut local_vm_a = make_vm(
        client_a,
        make_call_service_closure(call_service_in_a.clone(), call_service_out_a.clone()),
    );
    let call_service_in_b: Arc<Mutex<HashMap<String, JValue>>> = <_>::default();
    let call_service_out_b: Arc<Mutex<Vec<JValue>>> = <_>::default();
    let mut local_vm_b = make_vm(
        client_b,
        make_call_service_closure(call_service_in_b.clone(), call_service_out_b.clone()),
    );

    let script = r#"(call client_b ("return" "") [a b c])"#.to_string();
    let data = hashmap! {
        "client_b" => json!(client_b.to_string()),
        "a" => json!("a_value"),
        "b" => json!(["b1", "b2", "b3"]),
        "c" => json!({"c1": "c1_value", "c2": "c2_value"})
    };

    *call_service_in_a.lock() = data
        .iter()
        .map(|(key, value)| (key.to_string(), value.clone()))
        .collect();

    let particle = make_particle(
        client_a,
        call_service_in_a.clone(),
        script,
        None,
        &mut local_vm_a,
        false,
        Duration::from_secs(20),
    );

    let args = read_args(
        particle,
        client_b,
        &mut local_vm_b,
        call_service_out_b.clone(),
    );
    assert_eq!(data["a"], args[0]);
    assert_eq!(data["b"], args[1]);
    assert_eq!(data["c"], args[2]);
}
