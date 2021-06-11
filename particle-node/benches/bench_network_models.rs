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

#![allow(dead_code)]

use std::mem;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_std::task::{spawn, JoinHandle};
use eyre::WrapErr;
use futures::channel::mpsc;
use futures::future::BoxFuture;
use futures::{FutureExt, SinkExt};
use libp2p::PeerId;

use aquamarine::{
    AquamarineApi, AquamarineBackend, SendParticle, StepperEffects, VmConfig, VmPoolConfig, AVM,
};
use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use fluence_libp2p::types::{BackPressuredInlet, OneshotOutlet};
use fluence_libp2p::RandomPeerId;
use fs_utils::make_tmp_dir;
use kademlia::KademliaApi;
use particle_closures::{HostClosures, NodeInfo};
use particle_node::{ConnectionPoolCommand, Connectivity, KademliaCommand, NetworkApi};
use particle_protocol::{Contact, Particle};
use script_storage::ScriptStorageApi;
use server_config::ServicesConfig;
use std::convert::identity;
use test_utils::{make_swarms_with_mocked_vm, now_ms, EasyVM};

pub const TIMEOUT: Duration = Duration::from_secs(10);
pub const PARALLELISM: Option<usize> = Some(16);

pub async fn particles(n: usize) -> BackPressuredInlet<Particle> {
    let script = |_| String::from(r#"(call %init_peer_id% ("op" "identity") ["hello"] result)"#);
    particles_with_script(n, script).await
}

pub async fn particles_to_network(n: usize, peer_ids: Vec<PeerId>) -> BackPressuredInlet<Particle> {
    particles_with(n, |i, mut p| {
        // assuming that the particle will be always sent back to init_peer_id by mocked vm (EasyVM)
        p.init_peer_id = peer_ids[i % peer_ids.len()].clone();
        p
    })
    .await
}

pub async fn particles_with_script(
    n: usize,
    script: impl Fn(usize) -> String,
) -> BackPressuredInlet<Particle> {
    particles_with(n, |i, mut p| {
        p.script = script(i);
        p
    })
    .await
}

pub fn generate_particles(n: usize, modify: impl Fn(usize, Particle) -> Particle) -> Vec<Particle> {
    let last_particle = std::iter::once({
        let mut p = Particle::default();
        p.id = String::from("last");
        p
    });
    fn particle(n: usize) -> Particle {
        Particle {
            timestamp: now_ms() as u64,
            ttl: 10000,
            id: n.to_string(),
            ..<_>::default()
        }
    }

    (0..n)
        .map(|i| modify(i, particle(i)))
        .chain(last_particle)
        .collect()
}

pub async fn particles_with(
    n: usize,
    modify: impl Fn(usize, Particle) -> Particle,
) -> BackPressuredInlet<Particle> {
    let (mut outlet, inlet) = mpsc::channel(n * 2);

    let mut particles =
        futures::stream::iter(generate_particles(n, modify).into_iter().map(|p| Ok(p)));
    outlet.send_all(&mut particles).await.unwrap();
    mem::forget(outlet);

    inlet
}

pub fn kademlia_api() -> (KademliaApi, JoinHandle<()>) {
    use futures::StreamExt;

    let (outlet, mut inlet) = mpsc::unbounded();
    let api = KademliaApi { outlet };

    let handle = spawn(futures::future::poll_fn::<(), _>(move |cx| {
        use std::task::Poll;

        let mut wake = false;
        while let Poll::Ready(Some(cmd)) = inlet.poll_next_unpin(cx) {
            wake = true;
            // TODO: this shouldn't be called
            match cmd {
                KademliaCommand::AddContact { .. } => {}
                KademliaCommand::LocalLookup { out, .. } => out.send(vec![]).unwrap(),
                KademliaCommand::Bootstrap { out, .. } => out.send(Ok(())).unwrap(),
                KademliaCommand::DiscoverPeer { out, .. } => out.send(Ok(vec![])).unwrap(),
                KademliaCommand::Neighborhood { out, .. } => out.send(Ok(vec![])).unwrap(),
            }
        }

        if wake {
            cx.waker().wake_by_ref();
        }

        Poll::Pending
    }));

    (api, handle)
}

pub struct Stops(Vec<OneshotOutlet<()>>);
impl Stops {
    pub async fn cancel(self) {
        for stop in self.0 {
            stop.send(()).expect("send stop")
        }
    }
}

pub fn real_kademlia_api(network_size: usize) -> (KademliaApi, Stops, Vec<PeerId>) {
    let mut bootstrap_nodes = vec![];
    // create interconnected network of nodes
    let swarms = make_swarms_with_mocked_vm(network_size, identity, None, |bootstraps| {
        if bootstrap_nodes.is_empty() {
            bootstrap_nodes = bootstraps.clone();
        }

        bootstraps
    });

    let discoverer = make_swarms_with_mocked_vm(1, identity, None, identity)
        .into_iter()
        .next()
        .unwrap();

    // TODO: how to wait for bootstraps to settle before
    //       1) connecting `discoverer` to `swarms`
    //       2) continuing with tests/benchmark

    // connect discoverer node to the first of the interconnected
    // this way, discoverer will look up other nodes through that first node
    async_std::task::block_on(
        discoverer
            .connectivity
            .connection_pool
            .connect(Contact::new(
                swarms[0].peer_id,
                vec![swarms[0].multiaddr.clone()],
            )),
    );

    let (mut stops, peer_ids): (Vec<_>, _) =
        swarms.into_iter().map(|s| (s.outlet, s.peer_id)).unzip();
    stops.push(discoverer.outlet);

    let discoverer = discoverer.connectivity.kademlia;
    (discoverer, Stops(stops), peer_ids)
}

pub fn connection_pool_api(
    num_particles: usize,
    return_contact: bool,
) -> (ConnectionPoolApi, JoinHandle<()>) {
    use futures::StreamExt;

    let (outlet, mut inlet) = mpsc::unbounded();
    let api = ConnectionPoolApi {
        outlet,
        send_timeout: TIMEOUT,
    };

    let counter = AtomicUsize::new(0);

    let future = spawn(futures::future::poll_fn(move |cx| {
        use std::task::Poll;

        let mut wake = false;
        while let Poll::Ready(Some(cmd)) = inlet.poll_next_unpin(cx) {
            wake = true;

            match cmd {
                ConnectionPoolCommand::Connect { out, .. } => out.send(true).unwrap(),
                ConnectionPoolCommand::Send { out, .. } => {
                    let num = counter.fetch_add(1, Ordering::Relaxed);
                    out.send(true).unwrap();
                    if num == num_particles - 1 {
                        return Poll::Ready(());
                    }
                }
                ConnectionPoolCommand::Dial { out, .. } => out.send(None).unwrap(),
                ConnectionPoolCommand::Disconnect { out, .. } => out.send(true).unwrap(),
                ConnectionPoolCommand::IsConnected { out, .. } => out.send(true).unwrap(),
                ConnectionPoolCommand::GetContact { peer_id, out } => {
                    let contact = if return_contact {
                        Some(Contact::new(peer_id, vec![]))
                    } else {
                        None
                    };

                    out.send(contact).unwrap()
                }
                ConnectionPoolCommand::CountConnections { out, .. } => out.send(0).unwrap(),
                ConnectionPoolCommand::LifecycleEvents { .. } => {}
            }
        }

        if wake {
            cx.waker().wake_by_ref();
        }

        Poll::Pending
    }));

    (api, future)
}

pub fn aquamarine_api() -> (AquamarineApi, JoinHandle<()>) {
    use futures::StreamExt;

    let (outlet, mut inlet) = mpsc::channel(100);

    let api = AquamarineApi::new(outlet, TIMEOUT);

    let handle = spawn(futures::future::poll_fn::<(), _>(move |cx| {
        use std::task::Poll;

        let mut wake = false;
        while let Poll::Ready(Some(a)) = inlet.poll_next_unpin(cx) {
            wake = true;
            let (particle, ch) = a;
            ch.send(Ok(StepperEffects {
                particles: vec![SendParticle {
                    target: particle.init_peer_id,
                    particle,
                }],
            }))
            .unwrap();
        }

        if wake {
            cx.waker().wake_by_ref();
        }

        Poll::Pending
    }));

    (api, handle)
}

pub fn aquamarine_with_backend(
    pool_size: usize,
    delay: Option<Duration>,
) -> (AquamarineApi, JoinHandle<()>) {
    let config = VmPoolConfig {
        pool_size,
        execution_timeout: TIMEOUT,
    };
    let (backend, api): (AquamarineBackend<EasyVM>, _) = AquamarineBackend::new(config, delay);
    let handle = backend.start();

    (api, handle)
}

pub fn aquamarine_with_vm<C>(
    pool_size: usize,
    connectivity: C,
    local_peer_id: PeerId,
    interpreter: PathBuf,
) -> (AquamarineApi, JoinHandle<()>)
where
    C: Clone + Send + Sync + 'static + AsRef<KademliaApi> + AsRef<ConnectionPoolApi>,
{
    let tmp_dir = make_tmp_dir();

    let node_info = NodeInfo {
        external_addresses: vec![],
        node_version: "",
        air_version: "",
    };
    let script_storage_api = ScriptStorageApi {
        outlet: mpsc::unbounded().0,
    };
    let services_config = ServicesConfig::new(
        local_peer_id,
        tmp_dir.join("services"),
        <_>::default(),
        RandomPeerId::random(),
        RandomPeerId::random(),
    )
    .wrap_err("create service config")
    .unwrap();
    let host_closures =
        HostClosures::new(connectivity, script_storage_api, node_info, services_config);

    let pool_config = VmPoolConfig {
        pool_size,
        execution_timeout: TIMEOUT,
    };
    let vm_config = VmConfig {
        current_peer_id: local_peer_id,
        workdir: tmp_dir.join("workdir"),
        air_interpreter: interpreter,
        services_dir: tmp_dir.join("services_dir"),
        particles_dir: tmp_dir.join("particles_dir"),
    };
    let (stepper_pool, stepper_pool_api): (AquamarineBackend<AVM>, _) =
        AquamarineBackend::new(pool_config, (vm_config, host_closures.descriptor()));

    let handle = stepper_pool.start();

    (stepper_pool_api, handle)
}

pub async fn network_api(particles_num: usize) -> (NetworkApi, Vec<JoinHandle<()>>) {
    let particle_stream: BackPressuredInlet<Particle> = particles(particles_num).await;
    let particle_parallelism: usize = 1;
    let (kademlia, kad_handle) = kademlia_api();
    let (connection_pool, cp_handle) = connection_pool_api(1000, true);
    let bootstrap_frequency: usize = 1000;
    let particle_timeout: Duration = Duration::from_secs(5);

    let api: NetworkApi = NetworkApi::new(
        particle_stream,
        particle_parallelism,
        kademlia,
        connection_pool,
        bootstrap_frequency,
        particle_timeout,
    );
    (api, vec![cp_handle, kad_handle])
}

pub fn connectivity(
    num_particles: usize,
) -> (Connectivity, BoxFuture<'static, ()>, JoinHandle<()>) {
    let (kademlia, kad_handle) = kademlia_api();
    let (connection_pool, cp_handle) = connection_pool_api(num_particles, true);
    let connectivity = Connectivity {
        kademlia,
        connection_pool,
    };

    (connectivity, cp_handle.boxed(), kad_handle)
}

pub fn connectivity_with_real_kad(
    num_particles: usize,
    network_size: usize,
) -> (Connectivity, BoxFuture<'static, ()>, Stops, Vec<PeerId>) {
    let (kademlia, stops, peer_ids) = real_kademlia_api(network_size);
    let (connection_pool, cp_handle) = connection_pool_api(num_particles, false);
    let connectivity = Connectivity {
        kademlia,
        connection_pool,
    };

    (connectivity, cp_handle.boxed(), stops, peer_ids)
}

pub async fn process_particles(
    num_particles: usize,
    parallelism: Option<usize>,
    particle_timeout: Duration,
) {
    let (con, finish, kademlia) = connectivity(num_particles);
    let (aquamarine, aqua_handle) = aquamarine_api();
    let (sink, _) = mpsc::unbounded();

    let particle_stream: BackPressuredInlet<Particle> = particles(num_particles).await;
    let process = spawn(con.clone().process_particles(
        parallelism,
        particle_stream,
        aquamarine,
        sink,
        particle_timeout,
    ));
    finish.await;

    process.cancel().await;
    kademlia.cancel().await;
    aqua_handle.cancel().await;
}

pub async fn process_particles_with_vm(
    num_particles: usize,
    pool_size: usize,
    particle_parallelism: Option<usize>,
    particle_timeout: Duration,
    interpreter: PathBuf,
) {
    let peer_id = RandomPeerId::random();

    let (con, future, kademlia) = connectivity(num_particles);
    let (aquamarine, aqua_handle) =
        aquamarine_with_vm(pool_size, con.clone(), peer_id, interpreter);
    let (sink, _) = mpsc::unbounded();
    let particle_stream: BackPressuredInlet<Particle> = particles(num_particles).await;
    let process = spawn(con.clone().process_particles(
        particle_parallelism,
        particle_stream,
        aquamarine,
        sink,
        particle_timeout,
    ));
    future.await;

    process.cancel().await;
    kademlia.cancel().await;
    aqua_handle.cancel().await;
}

pub async fn process_particles_with_delay(
    num_particles: usize,
    pool_size: usize,
    call_delay: Option<Duration>,
    particle_parallelism: Option<usize>,
    particle_timeout: Duration,
) {
    let (con, future, kademlia) = connectivity(num_particles);
    let (aquamarine, aqua_handle) = aquamarine_with_backend(pool_size, call_delay);
    let (sink, _) = mpsc::unbounded();
    let particle_stream: BackPressuredInlet<Particle> = particles(num_particles).await;
    let process = spawn(con.clone().process_particles(
        particle_parallelism,
        particle_stream,
        aquamarine,
        sink,
        particle_timeout,
    ));
    future.await;

    process.cancel().await;
    kademlia.cancel().await;
    aqua_handle.cancel().await;
}
