/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
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

use async_std::{future::timeout, task};
use faas_api::{relay, service, Address, FunctionCall, Protocol};
use futures::select;
use janus_client::{Client, ClientCommand, ClientEvent, Transport};
use janus_libp2p::build_memory_transport;
use janus_server::ServerBehaviour;
use libp2p::{
    identity::{ed25519::Keypair, PublicKey::Ed25519},
    PeerId, Swarm,
};
use log::LevelFilter;
use once_cell::sync::Lazy;
use parity_multiaddr::Multiaddr;
use serde_json::{json, Value};
use std::time::Instant;
use std::{error::Error, time::Duration};
use uuid::Uuid;

static TIMEOUT: Lazy<Duration> = Lazy::new(|| Duration::from_secs(5));
static SHORT_TIMEOUT: Lazy<Duration> = Lazy::new(|| Duration::from_millis(100));
static KAD_TIMEOUT: Lazy<Duration> = Lazy::new(|| Duration::from_millis(500));

type Result<T> = core::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
struct ConnectedClient {
    client: Client,
    node: PeerId,
    node_address: Multiaddr,
}

impl ConnectedClient {
    pub fn client_address(&self) -> Address {
        Protocol::Client(self.client.peer_id.clone()).into()
    }

    pub fn relay_address(&self) -> Address {
        relay!(self.node.clone(), self.client.peer_id.clone())
    }

    pub fn send(&self, call: FunctionCall) {
        self.client.send(ClientCommand::Call {
            node: self.node.clone(),
            call,
        })
    }

    pub fn receive(&mut self) -> FunctionCall {
        let receive = self.client.receive_one();
        let result = task::block_on(timeout(*TIMEOUT, receive)).expect("get function call");

        if let Some(ClientEvent::FunctionCall { call, .. }) = result {
            call
        } else {
            panic!("Expected Some(FunctionCall), got {:?}", result)
        }
    }

    pub fn maybe_receive(&mut self) -> Option<FunctionCall> {
        let receive = self.client.receive_one();
        let result = task::block_on(timeout(*SHORT_TIMEOUT, receive))
            .ok()
            .flatten();

        result.and_then(|call| match call {
            ClientEvent::FunctionCall { call, .. } => Some(call),
            _ => None,
        })
    }
}

#[test]
// Send calls between clients through relays
fn send_call() {
    let (sender, mut receiver) = make_clients().expect("connect clients");

    let uuid = Uuid::new_v4().to_string();
    let call = FunctionCall {
        uuid: uuid.clone(),
        target: Some(receiver.relay_address()),
        reply_to: Some(sender.relay_address()),
        name: None,
        arguments: Value::Null,
    };

    sender.send(call);
    let received = receiver.receive();
    assert_eq!(received.uuid, uuid);
}

#[test]
// Provide service, and check that call reach it
fn call_service() {
    let service_id = "someserviceilike";
    let (mut provider, consumer) = make_clients().expect("connect clients");

    // Wait until Kademlia is ready // TODO: wait for event from behaviour instead?
    task::block_on(task::sleep(*KAD_TIMEOUT));

    let provide = provide_call(service_id, provider.relay_address());
    provider.send(provide);

    let call_service = service_call(service_id, consumer.relay_address());
    consumer.send(call_service.clone());

    let to_provider = provider.receive();

    assert_eq!(call_service.uuid, to_provider.uuid);
    assert_eq!(
        to_provider.target,
        Some(provider.client_address().extend(service!(service_id)))
    );
}

#[test]
// 1. Provide some service
// 2. Disconnect provider – service becomes unregistered
// 3. Check that calls to service fail
// 4. Provide same service again, via different provider
// 5. Check that calls to service succeed
fn provide_disconnect() {
    let service_id = "providedisconnect";

    let (mut provider, mut consumer) = make_clients().expect("connect clients");
    // Wait until Kademlia is ready // TODO: wait for event from behaviour instead?
    task::block_on(task::sleep(*KAD_TIMEOUT));

    // Register service
    let provide = provide_call(service_id, provider.relay_address());
    provider.send(provide);
    // Check there was no error // TODO: maybe send reply from relay?
    let error = provider.maybe_receive();
    assert_eq!(error, None);

    // Disconnect provider, service should be deregistered
    provider.client.stop();

    // Send call to the service, should fail
    let mut call_service = service_call(service_id, consumer.relay_address());
    call_service.name = Some("Send call to the service, should fail".into());
    consumer.send(call_service.clone());
    let error = consumer.receive();
    assert!(error.uuid.starts_with("error_"));

    // Register the service once again
    // let bootstraps = vec![provider.node_address.clone(), consumer.node_address.clone()];
    let mut provider = connect_client(provider.node_address).expect("connect provider");
    let provide = provide_call(service_id, provider.relay_address());
    provider.send(provide);
    let error = provider.maybe_receive();
    assert_eq!(error, None);

    // Send call to the service once again, should succeed
    call_service.name = Some("Send call to the service , should succeed".into());
    consumer.send(call_service.clone());
    let to_provider = provider.receive();

    assert_eq!(call_service.uuid, to_provider.uuid);
    assert_eq!(
        to_provider.target,
        Some(provider.client_address().extend(service!(service_id)))
    );
}

#[test]
// Receive error when there's not enough nodes to store service in DHT
fn provide_error() {
    let mut provider = make_client(vec![]).expect("connect client");
    let service_id = "failedservice";
    let provide = provide_call(service_id, provider.relay_address());
    provider.send(provide);
    let error = provider.receive();
    assert!(error.uuid.starts_with("error_"));
}

// --- Utility functions ---
fn provide_call(service_id: &str, reply_to: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(service!("provide")),
        reply_to: Some(reply_to),
        arguments: json!({ "service_id": service_id }),
        name: None,
    }
}

fn service_call(service_id: &str, consumer: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(service!(service_id)),
        reply_to: Some(consumer),
        arguments: Value::Null,
        name: None,
    }
}

fn uuid() -> String {
    Uuid::new_v4().to_string()
}

#[allow(dead_code)]
// Enables logging, filtering out unnecessary details
fn enable_logs() {
    use LevelFilter::*;

    env_logger::builder()
        .filter_level(Debug)
        .filter(Some("yamux::connection::stream"), Info)
        .filter(Some("tokio_threadpool"), Info)
        .filter(Some("tokio_reactor"), Info)
        .filter(Some("mio"), Info)
        .filter(Some("tokio_io"), Info)
        .filter(Some("soketto"), Info)
        .filter(Some("yamux"), Info)
        .filter(Some("multistream_select"), Info)
        .filter(Some("libp2p_secio"), Info)
        .filter(Some("libp2p_websocket::framed"), Info)
        .filter(Some("libp2p_ping"), Info)
        .filter(Some("libp2p_core::upgrade::apply"), Info)
        .filter(Some("libp2p_kad::kbucket"), Info)
        .filter(Some("libp2p_plaintext"), Info)
        .filter(Some("libp2p_identify::protocol"), Info)
        .init();
}

fn connect_client(node_address: Multiaddr) -> Result<ConnectedClient> {
    use core::result::Result;
    use std::io::{Error, ErrorKind};

    let connect = async move {
        let (mut client, _) = Client::connect_with(node_address.clone(), Transport::Memory)
            .await
            .expect("sender connected");
        let result: Result<_, Error> =
            if let Some(ClientEvent::NewConnection { peer_id, .. }) = client.receive_one().await {
                Ok(ConnectedClient {
                    client,
                    node: peer_id,
                    node_address,
                })
            } else {
                Err(ErrorKind::ConnectionAborted.into())
            };

        result
    };
    Ok(task::block_on(timeout(*TIMEOUT, connect))??)
}

fn make_client(bootstraps: Vec<Multiaddr>) -> Result<ConnectedClient> {
    let (node, addr1, mut swarm1) = create_swarm(bootstraps);
    swarm1.dial_bootstrap_nodes();

    task::spawn(async move { swarm1.next().await });

    let connect = async move {
        let (mut client, _) = Client::connect_with(addr1.clone(), Transport::Memory)
            .await
            .expect("sender connected");
        client.receive_one().await;

        ConnectedClient {
            client,
            node,
            node_address: addr1,
        }
    };
    Ok(task::block_on(timeout(*TIMEOUT, connect))?)
}

fn make_clients() -> Result<(ConnectedClient, ConnectedClient)> {
    use futures_util::FutureExt;
    use libp2p::core::ConnectedPoint::Dialer;
    use libp2p::swarm::SwarmEvent::{Behaviour, ConnectionEstablished};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let (peer_id1, addr1, mut swarm1) = create_swarm(vec![]);
    let (peer_id2, addr2, mut swarm2) = create_swarm(vec![addr1.clone()]);
    let (peer_id3, addr3, mut swarm3) = create_swarm(vec![addr1.clone(), addr2.clone()]);

    log::debug!("peer_id1: {}", peer_id1.to_base58());
    log::debug!("peer_id2: {}", peer_id2.to_base58());
    log::debug!("peer_id3: {}", peer_id3.to_base58());

    swarm2.dial_bootstrap_nodes();
    swarm3.dial_bootstrap_nodes();
    Swarm::dial_addr(&mut swarm1, addr2.clone()).expect("swarm1 dial addr2");
    Swarm::dial_addr(&mut swarm1, addr3.clone()).expect("swarm1 dial addr3");
    Swarm::dial_addr(&mut swarm2, addr3).expect("swarm2 dial addr3");

    let connected = Arc::new(AtomicUsize::new(0));
    let shared_connected = connected.clone();
    let _swarms_handle = task::spawn(async move {
        let connected = shared_connected;
        loop {
            let mut future1 = Swarm::next_event(&mut swarm1).boxed().fuse();
            let mut future2 = Swarm::next_event(&mut swarm2).boxed().fuse();
            let mut future3 = Swarm::next_event(&mut swarm3).boxed().fuse();

            select!(
                event = future1 => {
                    match event {
                        c@ConnectionEstablished { endpoint: Dialer { .. }, .. } => {
                            log::debug!("event: {:?}", c);
                            connected.fetch_add(1, Ordering::SeqCst);
                        },
                        Behaviour(event) => {
                            log::debug!("swarm1: behaviour event: {:?}", event)
                        },
                        _ => {}
                    }
                },
                event = future2 => {
                    match event {
                        c@ConnectionEstablished { endpoint: Dialer { .. }, .. } => {
                            log::debug!("event: {:?}", c);
                            connected.fetch_add(1, Ordering::SeqCst);
                        },
                        Behaviour(event) => {
                            log::debug!("swarm2: behaviour event: {:?}", event)
                        },
                        _ => {}
                    }
                },
                event = future3 => {
                    match event {
                        c@ConnectionEstablished { endpoint: Dialer { .. }, .. } => {
                            log::debug!("event: {:?}", c);
                            connected.fetch_add(1, Ordering::SeqCst);
                        },
                        Behaviour(event) => {
                            log::debug!("swarm3: behaviour event: {:?}", event)
                        },
                        _ => {}
                    }
                },
            )
        }
    });

    let now = Instant::now();
    while connected.load(Ordering::SeqCst) < 6 {}
    log::debug!("Connection took {}s", now.elapsed().as_secs_f32());

    let connect = async move {
        let (mut first, _) = Client::connect_with(addr1.clone(), Transport::Memory)
            .await
            .expect("first connected");
        first.receive_one().await;

        let first = ConnectedClient {
            client: first,
            node: peer_id1,
            node_address: addr1,
        };

        let (mut second, _) = Client::connect_with(addr2.clone(), Transport::Memory)
            .await
            .expect("second connected");
        second.receive_one().await;

        let second = ConnectedClient {
            client: second,
            node: peer_id2,
            node_address: addr2,
        };

        (first, second)
    };

    Ok(task::block_on(timeout(*TIMEOUT, connect))?)
}

fn create_swarm(bootstraps: Vec<Multiaddr>) -> (PeerId, Multiaddr, Swarm<ServerBehaviour>) {
    use libp2p::{core::multiaddr::Protocol, identity};
    use rand::random;

    let kp = Keypair::generate();
    let public_key = Ed25519(kp.public());
    let peer_id = PeerId::from(public_key);

    let mut swarm: Swarm<ServerBehaviour> = {
        use identity::Keypair::Ed25519;

        let server = ServerBehaviour::new(kp.clone(), peer_id.clone(), Vec::new(), bootstraps);
        let transport = build_memory_transport(Ed25519(kp));

        Swarm::new(transport, server, peer_id.clone())
    };
    let port = 1 + random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    Swarm::listen_on(&mut swarm, addr.clone()).unwrap();

    (peer_id, addr, swarm)
}
