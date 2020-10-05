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
    enable_logs, make_swarms_with_cfg, test_module, uuid, ConnectedClient, KAD_TIMEOUT,
};

use fluence_app_service::TomlFaaSNamedModuleConfig;
use libp2p::PeerId;
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

fn call_script(
    node: &PeerId,
    reply_to: &PeerId,
    service_id: &'static str,
    arg_name: &'static str,
) -> String {
    format!(
        "((call ({} ({} fname) ({}) result_name)) (call ({} (csrvcid cfname) ({}) result_name)))",
        node, service_id, arg_name, reply_to, arg_name
    )
}

#[test]
fn config() {
    let config = json!(
        {
            "name": "test_three",
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": json!({}),
                "preopened_files": vec!["./tests/artifacts"],
                "mapped_dirs": json!({}),
            }
        }
    );
    let config: TomlFaaSNamedModuleConfig =
        serde_json::from_value(config).expect("parse from jvalue");
    let str = toml::to_string(&config).expect("serialize to toml");
}

#[test]
fn add_module() {
    enable_logs();

    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let config = json!(
        {
            "name": "test_three",
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": json!({}),
                "preopened_files": vec!["./tests/artifacts"],
                "mapped_dirs": json!({}),
            }
        }
    );
    let script = call_script(&client.node, &client.peer_id, "add_module", "module");
    let response = send_particle(
        &mut client,
        script,
        json!({"module": { "module": test_module(), "config": config }}),
    );

    println!("response: {:?}", response);
}

#[test]
fn create_service() {
    enable_logs();

    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let mut particle = Particle::default();
    particle.id = "123".to_string();
    particle.init_peer_id = client.peer_id.clone();
    particle.script = format!(
        "((call ({} (create ||) (field) result_name)))",
        client.peer_id
    );
    particle.data = json!({"field": "value"});
    client.send(particle.clone());

    if cfg!(debug_assertions) {
        // Account for slow VM in debug
        client.timeout = Duration::from_secs(60);
    }

    let response = client.receive();
    assert_eq!(response.id, particle.id);
    assert_eq!(response.data, particle.data);
}
