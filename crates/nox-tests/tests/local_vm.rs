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

use fluence_keypair::KeyPair;
use std::fs::create_dir;
use std::sync::Arc;
use std::time::Duration;

use aquamarine::ParticleDataStore;
use maplit::hashmap;
use serde_json::json;

use local_vm::{make_particle, make_vm, read_args};

#[tokio::test]
async fn make() {
    let keypair_a = KeyPair::generate_ed25519();
    let keypair_b = KeyPair::generate_ed25519();
    let client_a = keypair_a.get_peer_id();
    let client_b = keypair_b.get_peer_id();
    let tmp_dir = tempfile::tempdir().expect("Could not create tmp dir");
    let tmp_dir_path = tmp_dir.path();
    let data_store = ParticleDataStore::new(
        tmp_dir_path.join("particle"),
        tmp_dir_path.join("vault"),
        tmp_dir_path.join("anomaly"),
    );
    data_store
        .initialize()
        .await
        .expect("Could not initialize datastore");
    let data_store = Arc::new(data_store);

    let local_vm_path_a = tmp_dir_path.join("vm_a");
    let local_vm_path_b = tmp_dir_path.join("vm_b");
    create_dir(&local_vm_path_a).expect("Could not create tmp dir");
    create_dir(&local_vm_path_b).expect("Could not create tmp dir");

    let mut local_vm_a = make_vm(local_vm_path_a.as_path()).await;
    let mut local_vm_b = make_vm(local_vm_path_b.as_path()).await;

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
        data_store.clone(),
        false,
        Duration::from_secs(20),
        &keypair_a,
    )
    .await;

    let args = read_args(particle, client_b, &mut local_vm_b, data_store, &keypair_b)
        .await
        .expect("read args")
        .expect("read args");
    assert_eq!(data["a"], args[0]);
    assert_eq!(data["b"], args[1]);
    assert_eq!(data["c"], args[2]);
}
