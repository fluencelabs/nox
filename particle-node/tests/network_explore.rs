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
use test_utils::{
    create_greeting_service, make_swarms, read_args, test_module, test_module_cfg, timeout,
    ClientEvent, ConnectedClient, KAD_TIMEOUT,
};

use futures::executor::block_on;
use maplit::hashmap;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value as JValue;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
pub struct VmDescriptor {
    interface: JValue,
    blueprint_id: String,
    service_id: Option<String>,
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
    pub hash: String,
    #[serde(default)]
    pub interface: JValue,
    #[serde(default)]
    pub error: Option<String>,
}

#[test]
fn get_interfaces() {
    let swarms = make_swarms(10);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let service1 = create_greeting_service(&mut client);
    let service2 = create_greeting_service(&mut client);

    client.send_particle(
        r#"
        (seq
            (call relay ("srv" "get_interfaces") [] interfaces)
            (call client ("return" "") [interfaces])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let value = client.receive_args().into_iter().next().unwrap();
    let vm_descriptors: Vec<VmDescriptor> =
        serde_json::from_value(value).expect("deserialize vm descriptors");
    assert!(vm_descriptors
        .iter()
        .find(|d| d.service_id.as_ref().unwrap() == service1.id.as_str())
        .is_some());
    assert!(vm_descriptors
        .iter()
        .find(|d| d.service_id.as_ref().unwrap() == service2.id.as_str())
        .is_some());
}

#[test]
fn get_modules() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("dist" "add_module") [module_bytes module_config])
                (call relay ("dist" "get_modules") [] modules)
            )
            (call client ("return" "") [modules])
        )
        "#,
        hashmap! {
            "module_bytes" => json!(base64::encode(test_module())),
            "module_config" => test_module_cfg("greeting"),
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    let value = client.receive_args().into_iter().next().unwrap();
    let modules: Vec<ModuleDescriptor> = serde_json::from_value(value).unwrap();
    assert_eq!(modules[0].name.as_deref(), Some("greeting"));
    assert!(matches!(modules[0].interface, JValue::Object(_)));
}

#[test]
fn get_blueprints() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("dist" "add_blueprint") [blueprint] blueprint_id)
                (call relay ("dist" "get_blueprints") [] blueprints)
            )
            (call client ("return" "") [blueprints blueprint_id])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "blueprint" => json!({ "name": "blueprint", "dependencies": ["module_name"] }),
        },
    );

    let value = client.receive_args().into_iter().next().unwrap();
    let bp: Vec<Blueprint> = serde_json::from_value(value).expect("deserialize blueprint");
    assert_eq!(bp.len(), 1);
    assert_eq!(bp[0].name, "blueprint");
    assert_eq!(bp[0].dependencies[0], "module_name");
}

#[test]
fn explore_services() {
    let swarms = make_swarms(5);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("dht" "neighborhood") [relay] neighs_top)
                (seq
                    (fold neighs_top n
                        (seq
                            (call n ("dht" "neighborhood") [n] neighs_inner[])
                            (next n)
                        )
                    )
                    (fold neighs_inner ns
                        (seq
                            ; HACK: convert ns from iterable to a value
                            (call relay ("op" "identity") [ns] ns_wrapped)
                            (seq
                                (fold ns_wrapped.$[0]! n
                                    (seq
                                        (call n ("op" "identify") [] services[])
                                        (next n)
                                    )
                                )
                                (next ns)
                            )
                        )
                    )
                )
            )
            (seq
                (call relay ("op" "identity") [])
                (call client ("return" "") [services neighs_inner neighs_top])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );

    client.timeout = Duration::from_secs(120);
    let mut args = client.receive_args().into_iter();
    let services = args.next().unwrap();
    println!("{}", services);

    println!("neighborhood: {}", args.next().unwrap());
}

#[test]
fn explore_services_fixed() {
    let swarms = make_swarms(5);
    sleep(KAD_TIMEOUT);

    // language=Clojure
    let script = r#"
        (seq
            (call relayId ("op" "identity") [])
            (fold peers p
                (par
                    (seq
                        (call p ("srv" "get_interfaces") [] interfaces[])
                        (seq
                            (call relayId ("op" "identity") [])
                            (call %init_peer_id% ("return" "") [p interfaces])
                        )
                    )
                    (next p)
                )
            )
        )
    "#;

    let peers = swarms.iter().skip(1);
    for peer in peers {
        let mut client = ConnectedClient::connect_to(peer.1.clone()).expect("connect client");
        create_greeting_service(&mut client);
    }

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let peers: Vec<_> = swarms.iter().skip(1).map(|s| s.0.to_string()).collect();
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
        if let Some(Some(event)) = block_on(timeout(Duration::from_secs(1), receive)).ok() {
            match event {
                ClientEvent::Particle { particle, .. } => {
                    let args = read_args(particle, &client.peer_id);
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
            .expect("find node with that peer id");

        let _: Vec<Vec<VmDescriptor>> = serde_json::from_value(interface).unwrap();
    }
}
