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
use fluence_faas::RawModuleConfig;
use libp2p::{identity::PublicKey::Ed25519, PeerId};
use parity_multiaddr::Multiaddr;
use serde_json::json;
use serde_json::Value;
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
                Protocol::Signature(receiver.sign("/incorrect/path".as_bytes()))
            } else {
                p
            }
        })
        .collect();

    let uuid = uuid();
    let call = FunctionCall {
        uuid: uuid.clone(),
        target: Some(target),
        module: None,
        fname: None,
        arguments: Value::Null,
        reply_to: Some(sender.relay_addr()),
        name: None,
        sender: sender.relay_addr(),
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
        uuid: uuid.clone(),
        target: Some(target),
        module: None,
        fname: None,
        arguments: Value::Null,
        reply_to: Some(sender.relay_addr()),
        name: None,
        sender: sender.relay_addr(),
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
    enable_logs();

    let service_id = "someserviceilike";
    let (mut provider, consumer) = ConnectedClient::make_clients().expect("connect clients");

    // Wait until Kademlia is ready // TODO: wait for event from behaviour instead?
    sleep(KAD_TIMEOUT);

    let provide = provide_call(service_id, provider.relay_addr(), provider.node_addr());
    provider.send(provide);

    let call_service = service_call(provider!(service_id), consumer.relay_addr(), service_id);
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

    let provide = provide_call(service_id, provider.relay_addr(), provider.node_addr());
    provider.send(provide);

    let call_service = service_call(provider!(service_id), consumer.relay_addr(), service_id);
    consumer.send(call_service);

    let to_provider = provider.receive();
    assert_eq!(to_provider.reply_to, Some(consumer.relay_addr()));

    let reply = reply_call(to_provider.reply_to.unwrap(), provider.relay_addr());
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
    let provide = provide_call(service_id, provider.relay_addr(), provider.node_addr());
    provider.send(provide);
    // Check there was no error // TODO: maybe send reply from relay?
    let error = provider.maybe_receive();
    assert_eq!(error, None);

    // Disconnect provider, service should be deregistered
    provider.client.stop();

    // Send call to the service, should fail
    let mut call_service = service_call(provider!(service_id), consumer.relay_addr(), service_id);
    call_service.name = Some("Send call to the service, should fail".into());
    consumer.send(call_service.clone());
    let error = consumer.receive();
    assert!(error.uuid.starts_with("error_"));

    // Register the service once again
    // let bootstraps = vec![provider.node_address.clone(), consumer.node_address.clone()];
    let mut provider =
        ConnectedClient::connect_to(provider.node_address).expect("connect provider");
    let provide = provide_call(service_id, provider.relay_addr(), provider.node_addr());
    provider.send(provide);
    let error = provider.maybe_receive();
    assert_eq!(error, None);

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
    let provide = provide_call(service_id, provider.relay_addr(), provider.node_addr());
    provider.send(provide);
    let error = provider.receive();
    assert!(error.uuid.starts_with("error_"));
}

#[test]
fn reconnect_provide() {
    let service_id = "popularservice";
    let swarms = make_swarms(5);
    sleep(KAD_TIMEOUT);
    let consumer = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect consumer");

    for _i in 1..20 {
        for swarm in swarms.iter() {
            let provider = ConnectedClient::connect_to(swarm.1.clone()).expect("connect provider");
            let call = provide_call(service_id, provider.relay_addr(), provider.node_addr());
            provider.send(call);
            sleep(SHORT_TIMEOUT);
        }
    }

    sleep(SHORT_TIMEOUT);

    let mut provider = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect provider");
    let call = provide_call(service_id, provider.relay_addr(), provider.node_addr());
    provider.send(call);

    sleep(KAD_TIMEOUT);

    let call_service = service_call(provider!(service_id), consumer.relay_addr(), service_id);
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
    let call = certificates_call(peer_id, consumer.relay_addr(), consumer.node_addr());
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
    let call = add_certificates_call(
        peer_id.clone(),
        client.relay_addr(),
        client.node_addr(),
        vec![cert.clone()],
    );
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
    client.send(call.clone());

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

    let mut identify_call = service_call(consumer.node_addr(), consumer.relay_addr(), "identify");
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
fn get_interface() {
    let ipfs_rpc = include_bytes!("artifacts/ipfs_node.wasm");
    let module: RawModuleConfig = toml::from_str(
        r#"
        name = "ipfs_node.wasm"
        mem_pages_count = 100
        logger_enabled = true

        [imports]
        mysql = "/usr/bin/mysql"
        ipfs = "/usr/local/bin/ipfs"

        [wasi]
        envs = []
        preopened_files = ["./artifacts"]
        mapped_dirs = { "tmp" = "./artifacts" }
    "#,
    )
    .expect("parse module config");

    println!("config is {:?}", module);

    let swarms = make_swarms_with(
        5,
        |bs, maddr| {
            let mut config = SwarmConfig::new(bs, maddr);
            config.wasm_modules = vec![(module.clone(), ipfs_rpc.to_vec())];
            create_swarm(config)
        },
        create_memory_maddr,
        true,
    );
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect consumer");
    let mut call = service_call(client.node_addr(), client.relay_addr(), "get_interface");
    let msg_id = uuid();
    call.arguments = json!({ "msg_id": msg_id });
    client.send(call.clone());
    let received = client.receive();
    println!("received: {:?}", received);
}
