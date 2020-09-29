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
use serde_json::json;
use test_utils::{make_swarms_with_cfg, ConnectedClient};

#[test]
fn create_service() {
    let swarms = make_swarms_with_cfg(3, |cfg| cfg.with_aquamarine("aquamarine_call.wasm"));
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let mut particle = Particle::default();
    particle.id = "123".to_string();
    particle.init_peer_id = client.peer_id.clone();
    particle.script = r#"((call ((peer_part (PeerId ())) (fn_part (create ||)) (args |{"kak":"dela"}|) (result_name RESNAME))))"#.to_string();
    particle.data = json!({"kak": "dela"});
    client.send(particle.clone());
    let response = client.receive();
    assert_eq!(response.id, particle.id);
    assert_eq!(response.data, particle.data);
}
