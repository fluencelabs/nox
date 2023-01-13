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
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, Instant};

use eyre::{ContextCompat, WrapErr};
use futures::executor::block_on;
use itertools::Itertools;
use libp2p::core::Multiaddr;
use maplit::hashmap;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value as JValue;

use connected_client::{ClientEvent, ConnectedClient};
use created_swarm::make_swarms;
use local_vm::read_args;
use service_modules::{load_module, module_config};
use test_constants::KAD_TIMEOUT;
use test_utils::{create_service, timeout};

use crate::network::join_stream;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Service {
    blueprint_id: String,
    id: String,
    owner_id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Blueprint {
    pub name: String,
    pub id: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModuleDescriptor {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub invalid_file_name: Option<String>,
    #[serde(default)]
    pub interface: JValue,
    #[serde(default)]
    pub error: Option<String>,
}

#[test]
fn get_interfaces() {
    let swarms = make_swarms(1);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();
    let service1 = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load"),
    );
    let service2 = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load"),
    );

    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("srv" "list") [] services)
                (fold services s
                    (seq
                        (call relay ("srv" "get_interface") [s.$.id!] $interfaces)
                        (next s)
                    )
                )
            )
            (seq
                (canon client $interfaces #interfaces)
                (call client ("return" "") [services #interfaces])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let args = client.receive_args().wrap_err("receive args").unwrap();
    let mut args = args.into_iter();
    let services = args.next().unwrap();
    let services: Vec<Service> = serde_json::from_value(services)
        .wrap_err("deserialize services")
        .unwrap();
    assert!(services.iter().any(|d| d.id == service1.id));
    assert!(services.iter().any(|d| d.id == service2.id));

    let interfaces_count = args.next().unwrap().as_array().unwrap().len();
    assert_eq!(interfaces_count, 2);
}

#[test]
fn get_modules() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    client.send_particle(
        r#"
            (seq
                (seq
                    (call relay ("dist" "add_module") [module_bytes module_config])
                    (seq
                        (call relay ("dist" "list_modules") [] modules)
                        (fold modules m
                            (seq
                                (call relay ("dist" "get_module_interface") [m.$.hash!] $interfaces)
                                (next m)
                            )
                        )               
                    )                    
                )
                (seq
                    (canon client $interfaces #interfaces)
                    (call client ("return" "") [modules #interfaces])
                )
            )
        "#,
        hashmap! {
            "module_bytes" => json!(base64::encode(load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"))),
            "module_config" => module_config("greeting"),
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let value = client.receive_args().wrap_err("receive args").unwrap();
    let mut iter = value.into_iter();
    let modules = iter.next().unwrap();
    let modules: Vec<ModuleDescriptor> = serde_json::from_value(modules).unwrap();
    // Now we have 3 modules: default sqlite3+spell and greeting
    assert_eq!(modules.len(), 3);
    assert!(modules
        .into_iter()
        .any(|m| m.name.as_deref() == Some("greeting")));

    let interfaces = iter.next();
    assert_eq!(interfaces.is_some(), true);
}

#[test]
fn list_blueprints() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let bytes = b"module";
    let raw_hash = blake3::hash(bytes).to_hex();
    let hash = format!("hash:{raw_hash}");
    let name = "name:module".to_string();
    client.send_particle(
        r#"
        (seq
            (call relay ("dist" "add_module") [module_bytes module_config] module_hash)
            (seq
                (seq
                    (call relay ("dist" "add_blueprint") [blueprint] blueprint_id)
                    (call relay ("dist" "list_blueprints") [] blueprints)
                )
                (call client ("return" "") [blueprints module_hash])
            )
        )
        "#,
        hashmap! {
            "module_bytes" => json!(base64::encode(bytes)),
            "module_config" => json!(module_config("module")),
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "blueprint" => json!({ "name": "blueprint", "dependencies": [ hash, name ] }),
        },
    );

    let args = client.receive_args().wrap_err("receive args").unwrap();
    let mut args = args.into_iter();
    let value = args.next().unwrap();
    let blueprints: Vec<Blueprint> = serde_json::from_value(value)
        .wrap_err("deserialize blueprint")
        .unwrap();

    // Now we have 2 blueprints: the first is for default spell service and the second is recent
    assert_eq!(blueprints.len(), 2);
    let bp = blueprints
        .into_iter()
        .find(|b| b.name == "blueprint")
        .unwrap();
    assert_eq!(bp.dependencies.len(), 2);
    assert_eq!(bp.dependencies[0], hash);
    // name:$name should've been converted to hash:$hash
    assert_eq!(bp.dependencies[1], hash);

    let hash = args.next().unwrap();
    let hash: String = serde_json::from_value(hash).unwrap();
    assert_eq!(hash.as_str(), raw_hash.as_str());
}

#[test]
fn explore_services() {
    let swarms = make_swarms(5);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    // N - 1 neighborhood each with N - 1 elements.
    let total_neighs = (swarms.len() - 1) * (swarms.len() - 1);

    client.send_particle(
        format!(
            r#"
        (seq
            (seq
                (call relay ("kad" "neighborhood") [relay] neighs_top)
                (seq
                    (fold neighs_top n
                        (seq
                            (call n ("kad" "neighborhood") [n] $neighs_inner)
                            (next n)
                        )
                    )
                    (fold $neighs_inner ns
                        (par
                            (fold ns n
                                (par
                                    (call n ("peer" "identify") [] $external_addresses)
                                    (next n)
                                )
                            )
                            (next ns)
                        )
                    )
                )
            )
            (seq
                {}
                (seq
                    (canon client $neighs_inner #neighs_inner)
                    (call client ("return" "") [#joined_addresses #neighs_inner neighs_top])
                )
            )
        )
        "#,
            join_stream(
                "external_addresses",
                "relay",
                &total_neighs.to_string(),
                "joined_addresses"
            )
        )
        .as_str(),
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let args = client.receive_args().wrap_err("receive args").unwrap();
    let external_addrs = args.into_iter().next().unwrap();
    let external_addrs = external_addrs.as_array().unwrap();
    let mut external_addrs = external_addrs
        .iter()
        .map(|v| {
            let external_addrs = v.get("external_addresses").unwrap().as_array().unwrap();
            let maddr = external_addrs[0].as_str().unwrap();
            Multiaddr::from_str(maddr).unwrap()
        })
        .collect::<Vec<_>>();
    external_addrs.sort_unstable();
    external_addrs.dedup();
    let expected_addrs: Vec<_> = swarms
        .iter()
        .map(|s| s.multiaddr.clone())
        .sorted_unstable()
        .collect();
    assert_eq!(external_addrs, expected_addrs);
}

#[test]
fn explore_services_fixed() {
    let swarms = make_swarms(5);
    sleep(KAD_TIMEOUT);

    // language=Clojure
    let script = r#"
        (seq
            (call relayId ("op" "noop") [])
            (fold peers p
                (par
                    (seq
                        (seq
                            (call p ("srv" "list") [] $services)
                            (canon p $services #services)
                        )
                        (seq
                            (call relayId ("op" "noop") [])
                            (call %init_peer_id% ("return" "") [p #services])
                        )
                    )
                    (next p)
                )
            )
        )
    "#;

    let peers = swarms.iter().skip(1);
    for peer in peers {
        let mut client = ConnectedClient::connect_to(peer.multiaddr.clone())
            .wrap_err("connect client")
            .unwrap();
        create_service(
            &mut client,
            "tetraplets",
            load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
        );
    }

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let peers: Vec<_> = swarms
        .iter()
        .skip(1)
        .map(|s| s.peer_id.to_string())
        .collect();
    let data = hashmap! {
        "peers" => json!(peers),
        "clientId" => json!(client.peer_id.to_string()),
        "relayId" => json!(client.node.to_string()),
    };

    client.send_particle(script, data);

    let now = Instant::now();
    let tout = Duration::from_secs(10);
    let mut received = Vec::new();

    loop {
        let receive = client.receive_one();
        if let Ok(Some(event)) = block_on(timeout(Duration::from_secs(1), receive)) {
            match event {
                ClientEvent::Particle { particle, .. } => {
                    let args = read_args(particle, client.peer_id, &mut client.local_vm.lock())
                        .expect("read args")
                        .expect("no error");
                    received.push(args);
                }
                ClientEvent::NewConnection { .. } => {}
            }
        }

        if received.len() == peers.len() {
            // success, break
            break;
        }

        if now.elapsed() > tout {
            // failure, panic
            panic!(
                "Test timed out after {} secs, {}/{} results collected",
                tout.as_secs(),
                received.len(),
                peers.len()
            );
        }
    }

    assert_eq!(received.len(), peers.len());

    for (peer_id, interface) in received.into_iter().map(|v| {
        let mut iter = v.into_iter();
        (iter.next().unwrap(), iter.next().unwrap())
    }) {
        let peer_id = peer_id.as_str().unwrap();
        peers
            .iter()
            .find(|node| peer_id == node.as_str())
            .wrap_err("find node with that peer id")
            .unwrap();

        let _: Vec<Vec<Service>> = serde_json::from_value(interface).unwrap();
    }
}
