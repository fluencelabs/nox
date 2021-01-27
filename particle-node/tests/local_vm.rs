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
use test_utils::{make_particle, read_args};

use maplit::hashmap;
use serde_json::json;

#[test]
fn make() {
    let client_a = RandomPeerId::random();
    let client_b = RandomPeerId::random();

    let script = r#"(call client_b ("return" "") [a b c] void[])"#.to_string();
    let data = hashmap! {
        "client_b" => json!(client_b.to_string()),
        "a" => json!("a_value"),
        "b" => json!(["b1", "b2", "b3"]),
        "c" => json!({"c1": "c1_value", "c2": "c2_value"})
    };

    let particle = make_particle(client_a.clone(), data.clone(), script.clone());

    let args = read_args(particle, &client_b);
    assert_eq!(data["a"], args[0]);
    assert_eq!(data["b"], args[1]);
    assert_eq!(data["c"], args[2]);
}
