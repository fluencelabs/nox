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

use async_std::task;
use faas_api::{service, Address, FunctionCall};
use fluence_libp2p::{build_memory_transport, build_transport};
use fluence_server::ServerBehaviour;

use fluence_client::Transport;
use libp2p::{
    identity::{
        ed25519::{Keypair, PublicKey},
        PublicKey::Ed25519,
    },
    PeerId, Swarm,
};
use parity_multiaddr::Multiaddr;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use trust_graph::{Certificate, TrustGraph};
use uuid::Uuid;

/// Utility functions for tests.

pub(crate) type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;
pub(crate) static TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) static SHORT_TIMEOUT: Duration = Duration::from_millis(100);
pub(crate) static KAD_TIMEOUT: Duration = Duration::from_millis(500);

pub(crate) fn certificates_call(peer_id: PeerId, reply_to: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(service!("certificates")),
        reply_to: Some(reply_to),
        arguments: json!({ "peer_id": peer_id.to_string(), "msg_id": uuid() }),
        name: None,
    }
}

pub(crate) fn add_certificates_call(
    peer_id: PeerId,
    reply_to: Address,
    certs: Vec<Certificate>,
) -> FunctionCall {
    let certs: Vec<_> = certs.into_iter().map(|c| c.to_string()).collect();
    FunctionCall {
        uuid: uuid(),
        target: Some(service!("add_certificates")),
        reply_to: Some(reply_to),
        arguments: json!({
            "peer_id": peer_id.to_string(),
            "msg_id": uuid(),
            "certificates": certs
        }),
        name: None,
    }
}

pub(crate) fn provide_call(service_id: &str, reply_to: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(service!("provide")),
        reply_to: Some(reply_to),
        arguments: json!({ "service_id": service_id }),
        name: None,
    }
}

pub(crate) fn service_call(service_id: &str, consumer: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(service!(service_id)),
        reply_to: Some(consumer),
        arguments: Value::Null,
        name: None,
    }
}

pub(crate) fn reply_call(reply_to: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(reply_to),
        reply_to: None,
        arguments: Value::Null,
        name: Some("reply".into()),
    }
}

pub(crate) fn uuid() -> String {
    Uuid::new_v4().to_string()
}

pub(crate) fn get_cert() -> Certificate {
    use std::str::FromStr;

    Certificate::from_str(
        r#"11
1111
EqpwyPYjbRbGPcp7Q1UtSnkeCDG9x3JrY96strN4uaXv
4Td1uTWzqWp1PyUzoUZyvWNjgPWQKpMFDYeqzoAJSXHQtkVispifSrnnqBFM8yFPkgmSHwQ4kTuACBifjoRryvFK
18446744073709551615
1589892496362
DYVjCCtVPnJNEDfRDzYn6a2GKJ6Qn4FNVwDhEAQBvdQS
3Tt8UxBr2pixgMMbRM4gnJDkX3zH3NnS5q4A5fCj3taMLpS2QathgUqkW4KHysQLeRoGxy3JNVtYEWLsL6kySrqv
1621450096362
1589892496362
HFF3V9XXbhdTLWGVZkJYd9a7NyuD5BLWLdwc4EFBcCZa
38FUPbDMrrb1FaRoRTsupjqysaH3vvpJJgp9NxLFBjBYoU353bb6LkDZLDsNwvnpVysrs6TdHeZAAe3iXrJuGLkn
101589892496363
1589892496363
"#,
    )
    .expect("deserialize cert")
}

#[allow(dead_code)]
// Enables logging, filtering out unnecessary details
pub(crate) fn enable_logs() {
    use log::LevelFilter::{Debug, Info};

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

pub(crate) struct CreatedSwarm(pub PeerId, pub Multiaddr);
pub(crate) fn make_swarms(n: usize) -> Vec<CreatedSwarm> {
    make_swarms_with(
        n,
        |bs, maddr| create_swarm(bs, maddr, None, Transport::Memory),
        create_memory_maddr,
    )
}

pub(crate) fn make_swarms_with<F, M>(
    n: usize,
    create_swarm: F,
    mut create_maddr: M,
) -> Vec<CreatedSwarm>
where
    F: Fn(Vec<Multiaddr>, Multiaddr) -> (PeerId, Swarm<ServerBehaviour>),
    M: FnMut() -> Multiaddr,
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

    let (infos, swarms): (Vec<CreatedSwarm>, Vec<_>) = swarms.into_iter().unzip();

    let connected = Arc::new(AtomicUsize::new(0));
    let shared_connected = connected.clone();

    // Run this task in background to poll swarms
    task::spawn(async move {
        let start = Instant::now();
        let mut local_start = Instant::now();

        swarms
            .into_iter()
            .map(|mut s| {
                let connected = shared_connected.clone();
                task::spawn(async move {
                    loop {
                        let event = s.next_event().await;
                        if let ConnectionEstablished {
                            endpoint: Dialer { .. },
                            ..
                        } = event
                        {
                            connected.fetch_add(1, Ordering::SeqCst);
                            let total = connected.load(Ordering::Relaxed);
                            if total % 10 == 0 {
                                log::info!(
                                    "established {: <10} +{: <10} (= {:<5})",
                                    total,
                                    format_args!("{:.3}s", start.elapsed().as_secs_f32()),
                                    format_args!("{}ms", local_start.elapsed().as_millis())
                                );
                                local_start = Instant::now();
                            }
                        }
                    }
                })
            })
            .collect::<FuturesUnordered<_>>()
            .forward(futures::sink::drain::<()>())
            .await
            .expect("drain");
    });

    let now = Instant::now();
    while connected.load(Ordering::SeqCst) < (n * (n - 1)) {}
    log::info!("Connection took {}s", now.elapsed().as_secs_f32());

    infos
}

#[derive(Default, Clone)]
pub(crate) struct Trust {
    pub(crate) root_weights: Vec<(PublicKey, u32)>,
    pub(crate) certificates: Vec<Certificate>,
    pub(crate) cur_time: Duration,
}

pub(crate) fn create_swarm(
    bootstraps: Vec<Multiaddr>,
    listen_on: Multiaddr,
    trust: Option<Trust>,
    transport: Transport,
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
        let server = ServerBehaviour::new(
            kp.clone(),
            peer_id.clone(),
            vec![listen_on.clone()],
            trust_graph,
            bootstraps,
        );
        match transport {
            Transport::Memory => {
                Swarm::new(build_memory_transport(Ed25519(kp)), server, peer_id.clone())
            }
            Transport::Network => Swarm::new(
                build_transport(Ed25519(kp), Duration::from_secs(10)),
                server,
                peer_id.clone(),
            ),
        }
    };

    Swarm::listen_on(&mut swarm, listen_on).unwrap();

    (peer_id, swarm)
}

pub fn create_memory_maddr() -> Multiaddr {
    use libp2p::core::multiaddr::Protocol;

    let port = 1 + rand::random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    addr
}
