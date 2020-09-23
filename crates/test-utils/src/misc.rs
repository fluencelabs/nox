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
use fluence_libp2p::{build_memory_transport, build_transport};
use particle_server::{BootstrapConfig, ServerBehaviour};

use crate::AQUAMARINE;
use fluence_client::Transport;
use futures::StreamExt;
use libp2p::{
    identity::{
        ed25519::{Keypair, PublicKey},
        PublicKey::Ed25519,
    },
    PeerId, Swarm,
};
use parity_multiaddr::Multiaddr;
use particle_actors::ActorConfig;
use prometheus::Registry;
use rand::Rng;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use trust_graph::{Certificate, TrustGraph};
use uuid::Uuid;

/// Utility functions for tests.

pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;
pub static TIMEOUT: Duration = Duration::from_secs(30);
pub static SHORT_TIMEOUT: Duration = Duration::from_millis(100);
pub static KAD_TIMEOUT: Duration = Duration::from_millis(500);

pub fn uuid() -> String {
    Uuid::new_v4().to_string()
}

pub fn get_cert() -> Certificate {
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
pub fn enable_logs() {
    use log::LevelFilter::Info;

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
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
        .filter(Some("cranelift_codegen"), Info)
        .filter(Some("wasmer_wasi"), Info)
        .try_init()
        .ok();
}

#[derive(Debug)]
pub struct CreatedSwarm(
    pub PeerId,
    pub Multiaddr,
    // tmp dir, must be cleaned
    pub PathBuf,
);
pub fn make_swarms(n: usize) -> Vec<CreatedSwarm> {
    make_swarms_with(
        n,
        |bs, maddr| create_swarm(SwarmConfig::new(bs, maddr)),
        create_memory_maddr,
        true,
    )
}

pub fn make_swarms_with<F, M>(
    n: usize,
    mut create_swarm: F,
    mut create_maddr: M,
    wait_connected: bool,
) -> Vec<CreatedSwarm>
where
    F: FnMut(Vec<Multiaddr>, Multiaddr) -> (PeerId, Swarm<ServerBehaviour>, PathBuf),
    M: FnMut() -> Multiaddr,
{
    use futures::stream::FuturesUnordered;
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
            let (id, swarm, tmp) = create_swarm(addrs, addr.clone());
            (CreatedSwarm(id, addr.clone(), tmp), swarm)
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

    if wait_connected {
        let now = Instant::now();
        while connected.load(Ordering::SeqCst) < (n * (n - 1)) {}
        log::info!("Connection took {}s", now.elapsed().as_secs_f32());
    }

    infos
}

#[derive(Default, Clone, Debug)]
pub struct Trust {
    pub root_weights: Vec<(PublicKey, u32)>,
    pub certificates: Vec<Certificate>,
    pub cur_time: Duration,
}

#[derive(Clone, Debug)]
pub struct SwarmConfig<'a> {
    pub bootstraps: Vec<Multiaddr>,
    pub listen_on: Multiaddr,
    pub trust: Option<Trust>,
    pub transport: Transport,
    pub registry: Option<&'a Registry>,
    pub tmp_dir: Option<PathBuf>,
}

impl<'a> Default for SwarmConfig<'a> {
    fn default() -> Self {
        Self {
            bootstraps: <_>::default(),
            listen_on: Multiaddr::empty(),
            trust: <_>::default(),
            transport: Transport::Memory,
            registry: <_>::default(),
            tmp_dir: <_>::default(),
        }
    }
}

impl<'a> SwarmConfig<'a> {
    pub fn new(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr) -> Self {
        Self {
            bootstraps,
            listen_on,
            transport: Transport::Memory,
            ..<_>::default()
        }
    }

    pub fn with_trust(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr, trust: Trust) -> Self {
        let mut this = Self::new(bootstraps, listen_on);
        this.trust = Some(trust);
        this
    }
}

pub fn create_swarm(config: SwarmConfig<'_>) -> (PeerId, Swarm<ServerBehaviour>, PathBuf) {
    use libp2p::identity;
    #[rustfmt::skip]
    let SwarmConfig { bootstraps, listen_on, trust, transport, registry, .. } = config;

    let kp = Keypair::generate();
    let public_key = Ed25519(kp.public());
    let peer_id = PeerId::from(public_key);

    let tmp = config.tmp_dir.unwrap_or_else(make_tmp_dir);

    let mut swarm: Swarm<ServerBehaviour> = {
        use identity::Keypair::Ed25519;

        let root_weights: &[_] = trust.as_ref().map_or(&[], |t| &t.root_weights);
        let mut trust_graph = TrustGraph::new(root_weights.to_vec());
        if let Some(trust) = trust {
            for cert in trust.certificates.into_iter() {
                trust_graph.add(cert, trust.cur_time).expect("add cert");
            }
        }

        let actor_config = ActorConfig::new(tmp.clone(), vec![], "aquamarine".to_string())
            .expect("create actor config");

        put_aquamarine(actor_config.modules_dir.clone());

        let server = ServerBehaviour::new(
            kp.clone(),
            peer_id.clone(),
            vec![listen_on.clone()],
            trust_graph,
            registry,
            bootstraps,
            BootstrapConfig::zero(),
            actor_config,
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

    (peer_id, swarm, tmp)
}

pub fn create_memory_maddr() -> Multiaddr {
    use libp2p::core::multiaddr::Protocol;

    let port = 1 + rand::random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    addr
}

pub fn make_tmp_dir() -> PathBuf {
    use rand::distributions::Alphanumeric;

    let mut tmp = std::env::temp_dir();
    tmp.push("fluence_test/");
    let dir: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(16)
        .collect();
    tmp.push(dir);

    std::fs::create_dir_all(&tmp).expect("create tmp dir");

    tmp
}

pub fn remove_dir(dir: &PathBuf) {
    std::fs::remove_dir_all(&dir).unwrap_or_else(|_| panic!("remove dir {:?}", dir))
}

pub fn put_aquamarine(mut tmp: PathBuf) {
    tmp.push("aquamarine.wasm");
    std::fs::write(&tmp, AQUAMARINE)
        .expect(format!("fs::write aquamarine.wasm to {:?}", tmp).as_str())
}
