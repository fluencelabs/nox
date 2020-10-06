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

use particle_protocol::Particle;
use test_utils::{
    enable_logs, format_aqua, make_swarms_with_cfg, test_module, uuid, ConnectedClient, KAD_TIMEOUT,
};

use serde_json::{json, Value};
use std::thread::sleep;
use std::time::Duration;

fn send_particle(client: &mut ConnectedClient, script: String, data: Value) -> Particle {
    let mut particle = Particle::default();
    particle.id = uuid();
    particle.init_peer_id = client.peer_id.clone();
    particle.script = script;
    particle.data = data;
    client.send(particle.clone());

    if cfg!(debug_assertions) {
        // Account for slow VM in debug
        client.timeout = Duration::from_secs(60);
    }

    let response = client.receive();

    response
}

#[test]
fn add_module_blueprint() {
    enable_logs();

    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let module = "greeting";
    let config = json!(
        {
            "name": module,
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": json!({}),
                "preopened_files": vec!["/tmp"],
                "mapped_dirs": json!({}),
            }
        }
    );

    let script = format_aqua(format!(
        r#"(seq (
            (call (%current_peer_id% (add_module ||) (module_bytes module_config) module))
            (seq (
                (call (%current_peer_id% (add_blueprint ||) (blueprint) blueprint_id))
                (seq (
                    (call (%current_peer_id% (create ||) (blueprint_id) service_id))
                    (call ({} (|| ||) (service_id) client_result))
                ))
            ))
        ))"#,
        client.peer_id
    ));

    let response = send_particle(
        &mut client,
        script,
        json!({
            "module_bytes": test_module(),
            "module_config": config,
            "blueprint": { "name": "blueprint", "dependencies": [module] },
        }),
    );

    let service_id = response.data.get("service_id").unwrap().as_str().unwrap();
    let script = format_aqua(format!(
        r#"(seq (
            (call (%current_peer_id% ({} |greeting|) (my_name) greeting))
            (call ({} (|| ||) (greeting) client_result))
        ))"#,
        service_id, client.peer_id
    ));

    let response = send_particle(
        &mut client,
        script,
        json!({
            "my_name": "folex"
        }),
    );

    assert_eq!(
        response.data.get("greeting").unwrap().as_str().unwrap(),
        "Hi, folex!"
    )
}
