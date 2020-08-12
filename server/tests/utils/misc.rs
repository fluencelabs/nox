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
use faas_api::{Address, FunctionCall, Protocol};
use fluence_libp2p::{build_memory_transport, build_transport};
use fluence_server::{BootstrapConfig, ServerBehaviour};

use crate::utils::ConnectedClient;
use fluence_app_service::RawModuleConfig;
use fluence_client::Transport;
use fluence_server::app_service::{AppServicesConfig, Blueprint};
use libp2p::{
    identity::{
        ed25519::{Keypair, PublicKey},
        PublicKey::Ed25519,
    },
    PeerId, Swarm,
};
use parity_multiaddr::Multiaddr;
use prometheus::Registry;
use rand::Rng;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use trust_graph::{Certificate, TrustGraph};
use uuid::Uuid;

/// Utility functions for tests.

pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;
pub static TIMEOUT: Duration = Duration::from_secs(30);
pub static SHORT_TIMEOUT: Duration = Duration::from_millis(100);
pub static KAD_TIMEOUT: Duration = Duration::from_millis(500);

pub fn certificates_call(peer_id: PeerId, sender: Address, node: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(node),
        module: Some("certificates".into()),
        arguments: json!({ "peer_id": peer_id.to_string(), "msg_id": uuid() }),
        reply_to: Some(sender.clone()),
        sender,
        ..<_>::default()
    }
}

pub fn add_certificates_call(
    peer_id: PeerId,
    sender: Address,
    node: Address,
    certs: Vec<Certificate>,
) -> FunctionCall {
    let certs: Vec<_> = certs.into_iter().map(|c| c.to_string()).collect();
    FunctionCall {
        uuid: uuid(),
        target: Some(node),
        module: Some("add_certificates".into()),
        arguments: json!({
            "peer_id": peer_id.to_string(),
            "msg_id": uuid(),
            "certificates": certs
        }),
        reply_to: Some(sender.clone()),
        sender,
        ..<_>::default()
    }
}

pub fn provide_call(service_id: &str, sender: Address, node: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(node),
        module: Some("provide".into()),
        arguments: json!({ "name": service_id, "address": sender.clone() }),
        reply_to: Some(sender.clone()),
        sender,
        ..<_>::default()
    }
}

pub fn service_call<S>(target: Address, sender: Address, module: S) -> FunctionCall
where
    S: Into<String>,
{
    FunctionCall {
        uuid: uuid(),
        target: Some(target),
        module: Some(module.into()),
        arguments: Value::Null,
        reply_to: Some(sender.clone()),
        sender,
        ..<_>::default()
    }
}

pub fn faas_call<SM, SF>(
    target: Address,
    sender: Address,
    module: SM,
    function: SF,
    service_id: String,
) -> FunctionCall
where
    SM: Into<String>,
    SF: Into<String>,
{
    FunctionCall {
        uuid: uuid(),
        target: Some(target.append(Protocol::Hashtag(service_id))),
        module: Some(module.into()),
        reply_to: Some(sender.clone()),
        fname: Some(function.into()),
        sender,
        ..<_>::default()
    }
}

pub fn add_module_call(
    target: Address,
    sender: Address,
    module: &[u8],
    config: RawModuleConfig,
) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(target),
        module: Some("add_module".to_string()),
        reply_to: Some(sender.clone()),
        arguments: json!({ "bytes": base64::encode(module), "config": config }),
        sender,
        ..<_>::default()
    }
}

pub fn add_blueprint_call(target: Address, sender: Address, blueprint: &Blueprint) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(target),
        module: Some("add_blueprint".to_string()),
        reply_to: Some(sender.clone()),
        arguments: json!({ "blueprint": blueprint }),
        sender,
        ..<_>::default()
    }
}

pub fn create_service_call<S: AsRef<str>>(
    target: Address,
    sender: Address,
    blueprint_id: S,
) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(target),
        module: Some("create".to_string()),
        reply_to: Some(sender.clone()),
        arguments: json!({ "blueprint_id": blueprint_id.as_ref() }),
        sender,
        ..<_>::default()
    }
}

pub fn reply_call(target: Address, sender: Address) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(target),
        arguments: Value::Null,
        name: Some("reply".into()),
        sender,
        ..<_>::default()
    }
}

impl ConnectedClient {
    pub fn certificates_call(&self, peer_id: PeerId) -> FunctionCall {
        certificates_call(peer_id, self.relay_addr(), self.node_addr())
    }
    pub fn add_certificates_call(&self, peer_id: PeerId, certs: Vec<Certificate>) -> FunctionCall {
        add_certificates_call(peer_id, self.relay_addr(), self.node_addr(), certs)
    }
    pub fn provide(&mut self, service_id: &str) -> FunctionCall {
        let call = provide_call(service_id, self.relay_addr(), self.node_addr());
        self.send(call);
        self.receive()
    }
    pub fn service_call<S>(&self, target: Address, module: S) -> FunctionCall
    where
        S: Into<String>,
    {
        service_call(target, self.relay_addr(), module)
    }
    pub fn local_service_call<S>(&self, module: S) -> FunctionCall
    where
        S: Into<String>,
    {
        self.service_call(self.node_addr(), module)
    }
    pub fn faas_call<SM, SF>(
        &self,
        target: Address,
        module: SM,
        function: SF,
        service_id: String,
    ) -> FunctionCall
    where
        SM: Into<String>,
        SF: Into<String>,
    {
        faas_call(target, self.relay_addr(), module, function, service_id)
    }

    pub fn local_faas_call<SM, SF>(
        &self,
        module: SM,
        function: SF,
        service_id: String,
    ) -> FunctionCall
    where
        SM: Into<String>,
        SF: Into<String>,
    {
        self.faas_call(self.node_addr(), module, function, service_id)
    }

    pub fn add_module(&mut self, module: &[u8], config: RawModuleConfig) {
        let call = add_module_call(self.node_addr(), self.relay_addr(), module, config);
        self.send(call);
        let received = self.receive();
        assert!(
            received.arguments.get("ok").is_some(),
            "module add failed {:?}",
            received
        );
    }

    pub fn add_blueprint(&mut self, dependencies: Vec<String>) -> Blueprint {
        let blueprint = Blueprint::new(uuid(), uuid(), dependencies);
        let call = add_blueprint_call(self.node_addr(), self.relay_addr(), &blueprint);
        self.send(call);
        let received = self.receive();
        assert!(
            received.arguments.get("ok").is_some(),
            "blueprint add failed {:?}",
            received
        );

        blueprint
    }

    pub fn create_service<S: AsRef<str>>(&mut self, target: Address, blueprint_id: S) -> String {
        let call = create_service_call(target, self.relay_addr(), blueprint_id);
        self.send(call);
        let received = self.receive();
        received.arguments["result"]["service_id"]
            .as_str()
            .expect(format!("service creation failed: {:?}", received).as_str())
            .to_string()
    }

    pub fn create_service_local<S: AsRef<str>>(&mut self, blueprint_id: S) -> String {
        self.create_service(self.node_addr(), blueprint_id)
    }

    pub fn reply_call(&self, target: Address) -> FunctionCall {
        reply_call(target, self.relay_addr())
    }

    pub fn get_modules(&mut self) -> Vec<Value> {
        let call = self.local_service_call("get_available_modules");
        self.send(call);
        let received = self.receive();

        received.arguments["available_modules"]
            .as_array()
            .unwrap_or_else(|| panic!("get array from {:#?}", received))
            .clone()
    }
}

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
}

impl<'a> SwarmConfig<'a> {
    pub fn new(bootstraps: Vec<Multiaddr>, listen_on: Multiaddr) -> Self {
        Self {
            bootstraps,
            listen_on,
            trust: None,
            transport: Transport::Memory,
            registry: None,
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

    let tmp = make_tmp_dir();
    println!("tmp dir is {:?}", tmp);

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
            registry,
            BootstrapConfig::zero(),
            AppServicesConfig::new(&tmp, vec![], &tmp),
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

fn make_tmp_dir() -> PathBuf {
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
