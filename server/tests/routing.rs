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

#![cfg(test)]
#![recursion_limit = "512"]
#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

use crate::utils::*;
use faas_api::{peer, provider, FunctionCall, Protocol};
use fluence_app_service::RawModuleConfig;
use libp2p::{identity::PublicKey::Ed25519, PeerId};
use parity_multiaddr::Multiaddr;
use serde_json::json;
use serde_json::Value;
use std::path::PathBuf;
use std::str::FromStr;
use std::thread::sleep;
use trust_graph::{current_time, Certificate};

mod utils;

#[test]
// Send calls between clients through relays
fn send_call() {
    let (sender, mut receiver) = ConnectedClient::make_clients().expect("connect clients");

    let call = FunctionCall::reply(
        receiver.relay_addr(),
        sender.relay_addr(),
        Value::Null,
        None,
    );

    sender.send(call.clone());
    let received = receiver.receive();
    assert_eq!(received.uuid, call.uuid);

    // Check there is no more messages
    let bad = receiver.maybe_receive();
    assert_eq!(
        bad,
        None,
        "received unexpected message {}, previous was {}",
        bad.as_ref().unwrap().uuid,
        received.uuid
    );
}

#[test]
fn invalid_relay_signature() {
    let (mut sender, receiver) = ConnectedClient::make_clients().expect("connect clients");
    let target = receiver.relay_addr();
    // replace signature with an incorrect one
    let target = target
        .protocols()
        .into_iter()
        .map(|p| {
            if let Protocol::Signature(_) = p {
                Protocol::Signature(receiver.sign(b"/incorrect/path"))
            } else {
                p
            }
        })
        .collect();

    let uuid = uuid();
    let call = FunctionCall {
        uuid,
        target: Some(target),
        reply_to: Some(sender.relay_addr()),
        sender: sender.relay_addr(),
        ..<_>::default()
    };

    sender.send(call);
    let reply = sender.receive();
    assert!(reply.uuid.starts_with("error_"));
    let err_msg = reply.arguments["reason"].as_str().expect("reason");
    assert!(err_msg.contains("invalid signature"));
}

#[test]
fn missing_relay_signature() {
    let (mut sender, receiver) = ConnectedClient::make_clients().expect("connect clients");
    let target = Protocol::Peer(receiver.node.clone()) / receiver.client_address();

    let uuid = uuid();
    let call = FunctionCall {
        uuid,
        target: Some(target),
        reply_to: Some(sender.relay_addr()),
        sender: sender.relay_addr(),
        ..<_>::default()
    };

    sender.send(call);
    let reply = sender.receive();
    assert!(reply.uuid.starts_with("error_"));
    let err_msg = reply.arguments["reason"].as_str().expect("reason");
    assert!(err_msg.contains("missing relay signature"));
}

#[test]
// Provide service, and check that call reach it
fn call_service() {
    let service_id = "someserviceilike";
    let (mut provider, consumer) = ConnectedClient::make_clients().expect("connect clients");

    // Wait until Kademlia is ready // TODO: wait for event from behaviour instead?
    sleep(KAD_TIMEOUT);

    let received = provider.provide(service_id);
    assert!(
        received.arguments.get("ok").is_some(),
        "provide failed: {:#?}",
        received
    );

    let call_service = consumer.service_call(provider!(service_id), service_id);
    consumer.send(call_service.clone());

    let to_provider = provider.receive();

    assert_eq!(
        call_service.uuid, to_provider.uuid,
        "Got: {:?}",
        to_provider
    );
    assert_eq!(to_provider.target, Some(provider.client_address()));
}

#[test]
fn call_service_reply() {
    let service_id = "plzreply";
    let (mut provider, mut consumer) = ConnectedClient::make_clients().expect("connect clients");

    // Wait until Kademlia is ready // TODO: wait for event from behaviour instead?
    sleep(KAD_TIMEOUT);

    let received = provider.provide(service_id);
    assert!(
        received.arguments.get("ok").is_some(),
        "provide failed: {:#?}",
        received
    );

    let call_service = consumer.service_call(provider!(service_id), service_id);
    consumer.send(call_service);

    let to_provider = provider.receive();
    assert_eq!(to_provider.reply_to, Some(consumer.relay_addr()));

    let reply = provider.reply_call(to_provider.reply_to.unwrap());
    provider.send(reply.clone());

    let to_consumer = consumer.receive();
    assert_eq!(reply.uuid, to_consumer.uuid, "Got: {:?}", to_consumer);
    assert_eq!(to_consumer.target, Some(consumer.client_address()));
}

#[test]
// 1. Provide some service
// 2. Disconnect provider – service becomes unregistered
// 3. Check that calls to service fail
// 4. Provide same service again, via different provider
// 5. Check that calls to service succeed
fn provide_disconnect() {
    let service_id = "providedisconnect";

    let swarms = make_swarms(10);
    sleep(KAD_TIMEOUT);
    let mut consumer = ConnectedClient::connect_to(swarms[3].1.clone()).expect("connect consumer");
    let mut provider = ConnectedClient::connect_to(swarms[4].1.clone()).expect("connect provider");

    // Register service
    let received = provider.provide(service_id);
    assert!(
        received.arguments.get("ok").is_some(),
        "provide failed: {:#?}",
        received
    );
    // Check there was no error // TODO: maybe send reply from relay?
    let error = provider.maybe_receive();
    assert_eq!(error, None);

    // Disconnect provider, service should be deregistered
    provider.client.stop();

    // Send call to the service, should fail
    let mut call_service = consumer.service_call(provider!(service_id), service_id);
    call_service.name = Some("Send call to the service, should fail".into());
    consumer.send(call_service.clone());
    let error = consumer.receive();
    assert!(error.uuid.starts_with("error_"));

    // Register the service once again
    // let bootstraps = vec![provider.node_address.clone(), consumer.node_address.clone()];
    let mut provider =
        ConnectedClient::connect_to(provider.node_address).expect("connect provider");
    let received = provider.provide(service_id);
    assert!(
        received.arguments.get("ok").is_some(),
        "provide failed: {:#?}",
        received
    );

    // Send call to the service once again, should succeed
    call_service.name = Some("Send call to the service , should succeed".into());
    consumer.send(call_service.clone());
    let to_provider = provider.receive();

    assert_eq!(
        call_service.uuid, to_provider.uuid,
        "Got: to_provider: {:#?}\ncall_service: {:#?}",
        to_provider, call_service
    );
    assert_eq!(to_provider.target, Some(provider.client_address()));
}

#[test]
// Receive error when there's not enough nodes to store service in DHT
fn provide_error() {
    let mut provider = ConnectedClient::new().expect("connect client");
    let service_id = "failedservice";
    let error = provider.provide(service_id);
    assert!(error.uuid.starts_with("error_"), "{:?}", error);
}

#[test]
fn reconnect_provide() {
    let service_id = "popularservice";
    let swarms = make_swarms(5);
    sleep(KAD_TIMEOUT);
    let consumer = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect consumer");

    for _i in 1..20 {
        for swarm in swarms.iter() {
            let mut provider =
                ConnectedClient::connect_to(swarm.1.clone()).expect("connect provider");
            let received = provider.provide(service_id);
            assert!(
                received.arguments.get("ok").is_some(),
                "provide failed: {:#?}",
                received
            );
        }
    }

    let mut provider = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect provider");
    let received = provider.provide(service_id);
    assert!(
        received.arguments.get("ok").is_some(),
        "provide failed: {:#?}",
        received
    );

    let call_service = consumer.service_call(provider!(service_id), service_id);
    consumer.send(call_service.clone());

    let to_provider = provider.receive();
    assert_eq!(to_provider.uuid, call_service.uuid);
}

#[test]
fn get_certs() {
    let cert = get_cert();
    let first_key = cert.chain.first().unwrap().issued_for.clone();
    let last_key = cert.chain.last().unwrap().issued_for.clone();

    let trust = Trust {
        root_weights: vec![(first_key, 1)],
        certificates: vec![cert.clone()],
        cur_time: current_time(),
    };

    let swarm_count = 5;
    let swarms = make_swarms_with(
        swarm_count,
        |bs, maddr| create_swarm(SwarmConfig::with_trust(bs, maddr, trust.clone())),
        create_memory_maddr,
        true,
    );
    sleep(KAD_TIMEOUT);
    let mut consumer = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect consumer");
    let peer_id = PeerId::from(Ed25519(last_key));
    let call = consumer.certificates_call(peer_id);
    consumer.send(call.clone());

    // If count is small, all nodes should fit in neighborhood, and all of them should reply
    for _ in 0..swarm_count {
        let reply = consumer.receive();
        assert_eq!(reply.arguments["msg_id"], call.arguments["msg_id"]);
        let reply_certs = &reply.arguments["certificates"][0]
            .as_str()
            .expect("get str cert");
        let reply_certs = Certificate::from_str(reply_certs).expect("deserialize cert");

        assert_eq!(reply_certs, cert);
    }
}

// TODO: test on get_certs error

#[test]
fn add_certs() {
    let cert = get_cert();
    let first_key = cert.chain.first().unwrap().issued_for.clone();
    let last_key = cert.chain.last().unwrap().issued_for.clone();

    let trust = Trust {
        root_weights: vec![(first_key, 1)],
        certificates: vec![],
        cur_time: current_time(),
    };

    let swarm_count = 5;
    let swarms = make_swarms_with(
        swarm_count,
        |bs, maddr| create_swarm(SwarmConfig::with_trust(bs, maddr, trust.clone())),
        create_memory_maddr,
        true,
    );
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect client");
    let peer_id = PeerId::from(Ed25519(last_key));
    let call = client.add_certificates_call(peer_id.clone(), vec![cert.clone()]);
    client.send(call.clone());

    // If count is small, all nodes should fit in neighborhood, and all of them should reply
    for _ in 0..swarm_count {
        let reply = client.receive();
        assert_eq!(
            reply.arguments["msg_id"], call.arguments["msg_id"],
            "{:#?}",
            reply
        );
    }

    for swarm in swarms {
        // repeat procedure twice to catch errors related to hanging requests
        for _ in 0..2 {
            let mut client =
                ConnectedClient::connect_to(swarm.1.clone()).expect("connect consumer");
            let call = certificates_call(peer_id.clone(), client.relay_addr(), client.node_addr());
            client.send(call.clone());

            for _ in 0..swarm_count {
                let reply = client.receive();
                assert_eq!(reply.arguments["msg_id"], call.arguments["msg_id"]);
                let reply_certs = &reply.arguments["certificates"][0]
                    .as_str()
                    .expect("get str cert");
                let reply_cert = Certificate::from_str(reply_certs).expect("deserialize cert");

                assert_eq!(reply_cert, cert);
            }
        }
    }
}

#[test]
fn add_certs_invalid_signature() {
    let mut cert = get_cert();
    let first_key = cert.chain.first().unwrap().issued_for.clone();
    let last_key = cert.chain.last().unwrap().issued_for.clone();

    let trust = Trust {
        root_weights: vec![(first_key, 1)],
        certificates: vec![],
        cur_time: current_time(),
    };

    let swarm_count = 5;
    let swarms = make_swarms_with(
        swarm_count,
        |bs, maddr| create_swarm(SwarmConfig::with_trust(bs, maddr, trust.clone())),
        create_memory_maddr,
        true,
    );
    sleep(KAD_TIMEOUT);

    // invalidate signature in last trust in `cert`
    let signature = &mut cert.chain.last_mut().unwrap().signature;
    signature.iter_mut().for_each(|b| *b = b.saturating_add(1));

    let mut client = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect consumer");
    let peer_id = PeerId::from(Ed25519(last_key));
    let call = add_certificates_call(peer_id, client.relay_addr(), client.node_addr(), vec![cert]);
    client.send(call);

    // check it's an error
    let reply = client.receive();
    assert!(reply.uuid.starts_with("error_"));
    let err_msg = reply.arguments["reason"].as_str().expect("reason");
    assert!(err_msg.contains("Signature is not valid"));
}

#[test]
fn identify() {
    let swarms = make_swarms(5);
    sleep(KAD_TIMEOUT);

    let mut consumer = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect consumer");

    let mut identify_call = consumer.service_call(consumer.node_addr(), "identify");
    let msg_id = uuid();
    identify_call.arguments = json!({ "msg_id": msg_id });
    consumer.send(identify_call.clone());

    fn check_reply(consumer: &mut ConnectedClient, swarm_addr: &Multiaddr, msg_id: &str) {
        let reply = consumer.receive();
        #[rustfmt::skip]
        let reply_msg_id = reply.arguments.get("msg_id").expect("not empty").as_str().expect("str");
        assert_eq!(reply_msg_id, msg_id);
        let addrs = reply.arguments["addresses"].as_array().expect("not empty");
        assert!(!addrs.is_empty());
        let addr: Multiaddr = addrs.first().unwrap().as_str().unwrap().parse().unwrap();
        assert_eq!(&addr, swarm_addr);
    }

    check_reply(&mut consumer, &swarms[1].1, &msg_id);

    for swarm in swarms {
        identify_call.target = Some(peer!(swarm.0.clone()));
        consumer.send(identify_call.clone());
        check_reply(&mut consumer, &swarm.1, &msg_id);
    }
}

#[test]
/// Call `get_interface` for two test modules, check they contain `greeting` and `empty` functions
fn get_interface() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);
    add_test_modules(swarms[0].1.clone());
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let dependencies = vec!["test_one".to_string(), "test_two".to_string()];
    let blueprint = client.add_blueprint(dependencies);
    let service_id = client.create_service_local(blueprint.id);
    let mut call = client.local_service_call("get_interface");
    let msg_id = uuid();
    call.arguments = json!({ "msg_id": msg_id, "service_id": service_id });
    client.send(call);
    let received = client.receive();

    let expected: Interface = serde_json::from_str(r#"{"modules":[{"name":"test_one","functions":[{"name":"empty","input_types":[],"output_types":[]},{"name":"greeting","input_types":["String"],"output_types":["String"]}]},{"name":"test_two","functions":[{"name":"empty","input_types":[],"output_types":[]},{"name":"greeting","input_types":["String"],"output_types":["String"]}]}]}"#).unwrap();
    let actual: Interface =
        serde_json::from_value(received.arguments["interface"].clone()).unwrap();

    assert_eq!(expected, actual);
}

#[test]
fn call_greeting() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);
    add_test_modules(swarms[0].1.clone());
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    for module in &["test_one", "test_two"] {
        let blueprint = client.add_blueprint(vec![module.to_string()]);
        let service_id = client.create_service_local(blueprint.id);
        let mut call = client.local_faas_call(*module, "greeting", service_id);

        // Pass arguments as an array
        let payload: String = "Hello".into();
        call.arguments = Value::Array(vec![payload.clone().into()]);
        client.send(call.clone());

        let received = client.receive();
        assert_eq!(
            &received.arguments["result"], &payload,
            "{:?}",
            received.arguments
        );

        // Pass arguments as a map
        let mut map = serde_json::Map::new();
        map.insert("anything".into(), payload.clone().into());
        call.arguments = Value::Object(map);
        client.send(call);

        let received = client.receive();
        assert_eq!(&received.arguments["result"], &payload);
    }

    remove_dir(&swarms[0].2)
}

#[test]
fn call_empty() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);
    add_test_modules(swarms[0].1.clone());
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let dependencies = vec!["test_one".to_string(), "test_two".to_string()];
    let blueprint = client.add_blueprint(dependencies.clone());
    let service_id = client.create_service_local(blueprint.id);

    for module in dependencies {
        let mut call = client.local_faas_call(module, "empty", service_id.clone());
        call.fname = Some("empty".into());

        client.send(call.clone());
        let received = client.receive();

        assert!(received.arguments.as_object().unwrap().is_empty());
    }

    remove_dir(&swarms[0].2)
}

#[test]
fn find_blueprint_provider() {
    let swarms = make_swarms(5);
    sleep(KAD_TIMEOUT);
    add_test_modules(swarms[0].1.clone());

    let payload = "payload".to_string();
    let module = "test_one";
    let mut consumer = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect consumer");
    let blueprint = consumer.add_blueprint(vec![module.to_string()]);
    let service_id = consumer.create_service(provider!(&blueprint.id), &blueprint.id);
    let mut call = consumer.faas_call(provider!(blueprint.id), module, "greeting", service_id);
    call.arguments = Value::Array(vec![Value::String(payload.clone())]);
    consumer.send(call);

    let received = consumer.receive();
    assert_eq!(&received.arguments["result"], &payload, "{:?}", received);
}

#[test]
fn get_interfaces() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);
    add_test_modules(swarms[0].1.clone());
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let dependencies = vec!["test_one".to_string(), "test_two".to_string()];
    let blueprint = client.add_blueprint(dependencies);
    let service_id1 = client.create_service_local(&blueprint.id);
    let service_id2 = client.create_service_local(&blueprint.id);
    let interfaces = client.get_active_interfaces();

    let expected: Interface = serde_json::from_str(r#"{"modules":[{"name":"test_one","functions":[{"name":"empty","input_types":[],"output_types":[]},{"name":"greeting","input_types":["String"],"output_types":["String"]}]},{"name":"test_two","functions":[{"name":"empty","input_types":[],"output_types":[]},{"name":"greeting","input_types":["String"],"output_types":["String"]}]}]}"#).unwrap();
    let actual1 = interfaces
        .iter()
        .find(|s| s.service_id == service_id1)
        .unwrap();
    assert_eq!(expected, actual1.service);

    let actual2 = interfaces
        .iter()
        .find(|s| s.service_id == service_id2)
        .unwrap();
    assert_eq!(expected, actual2.service);
}

#[test]
fn test_get_modules() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);
    add_test_modules(swarms[0].1.clone());
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let mut expected = vec!["test_one", "test_two"];
    let mut actual = client.get_available_modules();
    expected.sort();
    actual.sort();
    assert_eq!(expected, actual);
}

#[test]
#[rustfmt::skip]
fn add_module() {
    let config: RawModuleConfig = serde_json::from_value(json!(
        {
            "name": "test_three",
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": Vec::<()>::new(),
                "preopened_files": vec!["./tests/artifacts"],
                "mapped_dirs": json!({}),
            }
        }
    )).unwrap();

    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);
    add_test_modules(swarms[0].1.clone());
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    // Add new module to faas
    client.add_module(test_module().as_slice(), config.clone());

    let compare_sorted = |mut left: Vec<_>, mut right: Vec<_>| {
        left.sort();
        right.sort();
        assert_eq!(left, right)
    };
    
    // Check it is available
    compare_sorted(client.get_available_modules(), vec!["test_one", "test_two", "test_three"]);
    
    // Check it won't be duplicated
    client.add_module(test_module().as_slice(), config);
    compare_sorted(client.get_available_modules(), vec!["test_one", "test_two", "test_three"]);
    
    // Create a blueprint with that module
    let blueprint = client.add_blueprint(vec!["test_two".to_string(), "test_three".to_string()]);
    // Create a service from blueprint
    let service_id = client.create_service_local(blueprint.id);
    
    // Call new service
    let mut call = client.local_faas_call("test_three", "greeting", service_id);
    let payload = "Hello";
    call.arguments = Value::Array(vec![payload.to_string().into()]);
    client.send(call);
    let received = client.receive();
    assert_eq!(received.arguments["result"].as_str().unwrap(), payload);
}

fn swarms_with_tmp_dirs(n: usize, tmp_dirs: &[PathBuf]) -> Vec<CreatedSwarm> {
    let mut counter = 0;
    make_swarms_with(
        n,
        |bs, maddr| {
            let tmp_dir = tmp_dirs[counter].clone();
            let mut config = SwarmConfig::new(bs, maddr);
            config.tmp_dir = Some(tmp_dir);
            counter += 1;
            create_swarm(config)
        },
        create_memory_maddr,
        true,
    )
}

#[test]
fn restart_persistent_services() {
    let n = 3;
    let tmp_dirs = (0..n).map(|_| make_tmp_dir()).collect::<Vec<_>>();
    let swarms = swarms_with_tmp_dirs(n, tmp_dirs.as_slice());
    sleep(KAD_TIMEOUT);
    add_test_modules(swarms[0].1.clone());

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let dependencies = vec!["test_one".to_string(), "test_two".to_string()];
    let blueprint = client.add_blueprint(dependencies);
    for _ in 0..n {
        client.create_service_local(&blueprint.id);
    }

    assert_eq!(client.get_available_modules().len(), 2);
    assert_eq!(client.get_available_blueprints().len(), 1);
    assert_eq!(client.get_active_interfaces().len(), n);

    drop(swarms);

    let swarms = swarms_with_tmp_dirs(n, tmp_dirs.as_slice());
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    assert_eq!(client.get_available_modules().len(), 2);
    assert_eq!(client.get_available_blueprints().len(), 1);

    let mut attempts = 0;
    loop {
        sleep(SHORT_TIMEOUT * 5);
        let services = client.get_active_interfaces().len();
        if services == 3 {
            break;
        }

        attempts += 1;
        if attempts > 60 {
            // TODO: KARAUUUL~. 60 is too long.
            panic!(
                "Timed out waiting for 3 services to be created. Attempts {}/60, services {}/3",
                attempts, services
            );
        }
    }
}
