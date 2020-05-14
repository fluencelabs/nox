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
    identity::{
        ed25519::{Keypair, PublicKey},
        PublicKey::Ed25519,
    },
    PeerId, Swarm,
};
use log::LevelFilter;
use once_cell::sync::Lazy;
use parity_multiaddr::Multiaddr;
use rand::random;
use serde_json::{json, Value};
use std::str::FromStr;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::{error::Error, time::Duration};
use trust_graph::{Certificate, TrustGraph};
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
        let addr = relay!(self.node.clone(), self.client.peer_id.clone());
        let sig = self.client.key_pair.sign(addr.path().as_bytes());
        addr.append(Protocol::Signature(sig))
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

    assert_eq!(
        call_service.uuid, to_provider.uuid,
        "Got: {:?}",
        to_provider
    );
    assert_eq!(
        to_provider.target,
        Some(provider.client_address().extend(service!(service_id)))
    );
}

#[test]
fn call_service_reply() {
    let service_id = "plzreply";
    let (mut provider, mut consumer) = make_clients().expect("connect clients");

    // Wait until Kademlia is ready // TODO: wait for event from behaviour instead?
    task::block_on(task::sleep(*KAD_TIMEOUT));

    let provide = provide_call(service_id, provider.relay_address());
    provider.send(provide);

    let call_service = service_call(service_id, consumer.relay_address());
    consumer.send(call_service.clone());

    let to_provider = provider.receive();
    assert_eq!(to_provider.reply_to, Some(consumer.relay_address()));

    let reply = reply_call(to_provider.reply_to.unwrap());
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
    let mut provider = make_client().expect("connect client");
    let service_id = "failedservice";
    let provide = provide_call(service_id, provider.relay_address());
    provider.send(provide);
    let error = provider.receive();
    assert!(error.uuid.starts_with("error_"));
}

// TODO: test on invalid signature
// TODO: test on missing signature

#[test]
fn reconnect_provide() {
    let service_id = "popularservice";
    let swarms = make_swarms(10);
    task::block_on(task::sleep(*KAD_TIMEOUT));
    let consumer = connect_client(swarms[1].1.clone()).expect("connect consumer");

    for _i in 1..100 {
        for swarm in swarms.iter() {
            let provider = connect_client(swarm.1.clone()).expect("connect provider");
            let provide_call = provide_call(service_id, provider.relay_address());
            provider.send(provide_call);
            task::block_on(task::sleep(*SHORT_TIMEOUT));
        }
    }

    task::block_on(task::sleep(*SHORT_TIMEOUT));

    let mut provider = connect_client(swarms[0].1.clone()).expect("connect provider");
    let provide_call = provide_call(service_id, provider.relay_address());
    provider.send(provide_call);

    task::block_on(task::sleep(*KAD_TIMEOUT));

    let call_service = service_call(service_id, consumer.relay_address());
    consumer.send(call_service.clone());

    let to_provider = provider.receive();
    assert_eq!(to_provider.uuid, call_service.uuid);
}

#[test]
fn get_certs() {
    let cert = Certificate::from_str(
        r#"11
1111
GA9VsZa2Cw2RSZWfxWLDXTLnzVssAYPdrkwT1gHaTJEg
QPdFbzpN94d5tzwV4ATJXXzb31zC7JZ9RbZQ1pbgN3TTDnxuVckCmvbzvPaXYj8Mz7yrL66ATbGGyKqNuDyhXTj
18446744073709551615
1589250571718
D7p1FrGz35dd3jR7PiCT12pAZzxV5PFBcX7GdS9Y8JNb
61QT8JYsAWXjnmUcjJcuNnBHWfrn9pijJ4mX64sDX4N7Knet5jr2FzELrJZAAV1JDZQnATYpGf7DVhnitcUTqpPr
1620808171718
1589250571718
4cEQ2DYGdAbvgrAP96rmxMQvxk9MwtqCAWtzRrwJTmLy
xdHh499gCUD7XA7WLXqCR9ZXxQZFweongvN9pa2egVdC19LJR9814pNReP4MBCCctsGbLmddygT6Pbev1w62vDZ
1620808171719
1589250571719"#,
    )
    .expect("deserialize cert");

    let first_key = cert.chain.first().unwrap().issued_for.clone();
    let last_key = cert.chain.last().unwrap().issued_for.clone();

    fn current_time() -> Duration {
        Duration::from_millis(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        )
    }

    let trust = Trust {
        root_weights: vec![(first_key, 1)],
        certificates: vec![cert.clone()],
        cur_time: current_time(),
    };

    let swarm_count = 5;
    let swarms = make_swarms_with(swarm_count, |bs, maddr| {
        create_swarm(bs, maddr, Some(trust.clone()))
    });
    task::block_on(task::sleep(*KAD_TIMEOUT));
    let mut consumer = connect_client(swarms[1].1.clone()).expect("connect consumer");
    let peer_id = PeerId::from(Ed25519(last_key));
    let call = certificates_call(peer_id, consumer.relay_address());
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

// --- Utility functions ---
fn certificates_call(peer_id: PeerId, reply_to: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(service!("certificates")),
        reply_to: Some(reply_to),
        arguments: json!({ "peer_id": peer_id.to_string(), "msg_id": uuid() }),
        name: None,
    }
}

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

fn reply_call(reply_to: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(reply_to),
        reply_to: None,
        arguments: Value::Null,
        name: Some("reply".into()),
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
        .try_init()
        .ok();
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

fn make_client() -> Result<ConnectedClient> {
    let swarm = make_swarms(3).into_iter().next().unwrap();
    let CreatedSwarm(node, addr1) = swarm;

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

struct CreatedSwarm(PeerId, Multiaddr);
fn make_swarms(n: usize) -> Vec<CreatedSwarm> {
    make_swarms_with(n, |bs, maddr| create_swarm(bs, maddr, None))
}

fn make_swarms_with<F>(n: usize, create_swarm: F) -> Vec<CreatedSwarm>
where
    F: Fn(Vec<Multiaddr>, Multiaddr) -> (PeerId, Swarm<ServerBehaviour>),
{
    use futures::stream::FuturesUnordered;
    use futures_util::StreamExt;
    use libp2p::core::ConnectedPoint::Dialer;
    use libp2p::swarm::SwarmEvent::ConnectionEstablished;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let addrs = (0..n).map(|_| create_maddr()).collect::<Vec<_>>();

    let mut swarms = addrs
        .iter()
        .map(|addr| {
            #[rustfmt::skip]
            let addrs = addrs.iter().filter(|&a| a != addr).cloned().collect::<Vec<_>>();
            let (id, swarm) = create_swarm(addrs, addr.clone());
            (CreatedSwarm(id, addr.clone()), swarm)
        })
        .collect::<Vec<_>>();

    #[rustfmt::skip]
    swarms.iter_mut().for_each(|(_, s)| s.dial_bootstrap_nodes());

    let (infos, mut swarms): (Vec<CreatedSwarm>, Vec<_>) = swarms.into_iter().unzip();

    let connected = Arc::new(AtomicUsize::new(0));
    let shared_connected = connected.clone();
    let _swarms_handle = task::spawn(async move {
        let connected = shared_connected;
        let start = Instant::now();
        let mut local_start = Instant::now();
        loop {
            let swarms = swarms
                .iter_mut()
                .map(|s| Swarm::next_event(s))
                .collect::<FuturesUnordered<_>>();
            let mut swarms = swarms.fuse();

            select!(
                event = swarms.next() => {
                    if let Some(ConnectionEstablished { endpoint: Dialer { .. }, .. }) = event {
                        connected.fetch_add(1, Ordering::SeqCst);
                        let total = connected.load(Ordering::Relaxed);
                        if total % 10 == 0 {
                            log::trace!(
                                "established {: <10} +{: <10} (= {:<5})",
                                total, format_args!("{:.3}s", start.elapsed().as_secs_f32()), format_args!("{}ms", local_start.elapsed().as_millis())
                            );
                            local_start = Instant::now();
                        }
                    }
                },
            )
        }
    });

    let now = Instant::now();
    while connected.load(Ordering::SeqCst) < (n * (n - 1)) {}
    log::debug!("Connection took {}s", now.elapsed().as_secs_f32());

    infos
}

fn make_clients() -> Result<(ConnectedClient, ConnectedClient)> {
    let swarms = make_swarms(3);
    let mut swarms = swarms.into_iter();
    let CreatedSwarm(peer_id1, addr1) = swarms.next().expect("get swarm");
    let CreatedSwarm(peer_id2, addr2) = swarms.next().expect("get swarm");

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

#[derive(Default, Clone)]
struct Trust {
    root_weights: Vec<(PublicKey, u32)>,
    certificates: Vec<Certificate>,
    cur_time: Duration,
}

fn create_swarm(
    bootstraps: Vec<Multiaddr>,
    listen_on: Multiaddr,
    trust: Option<Trust>,
) -> (PeerId, Swarm<ServerBehaviour>) {
    use libp2p::identity;

    let kp = Keypair::generate();
    let public_key = Ed25519(kp.public());
    let peer_id = PeerId::from(public_key);

    let mut swarm: Swarm<ServerBehaviour> = {
        use identity::Keypair::Ed25519;

        let root_weights: &[_] = trust.as_ref().map_or(&[], |t| &t.root_weights);
        let mut trust_graph = TrustGraph::new(root_weights.to_vec());
        if let Some(trust) = trust {
            for cert in trust.certificates.into_iter() {
                trust_graph.add(cert, trust.cur_time).expect("add cert");
            }
        }
        let server = ServerBehaviour::new(kp.clone(), peer_id.clone(), trust_graph, bootstraps);
        let transport = build_memory_transport(Ed25519(kp));

        Swarm::new(transport, server, peer_id.clone())
    };

    Swarm::listen_on(&mut swarm, listen_on).unwrap();

    (peer_id, swarm)
}

fn create_maddr() -> Multiaddr {
    use libp2p::core::multiaddr::Protocol;

    let port = 1 + random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    addr
}
