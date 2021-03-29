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

use aquamarine::{
    AquaRuntime, AquamarineApi, AquamarineBackend, AquamarineVM, InterpreterOutcome, SendParticle,
    StepperEffects, VmConfig, VmPoolConfig,
};
use async_std::task;
use async_std::task::{spawn, JoinHandle};
use connection_pool::ConnectionPoolApi;
use criterion::async_executor::AsyncStdExecutor;
use criterion::{criterion_group, criterion_main, BatchSize};
use criterion::{BenchmarkId, Criterion, Throughput};
use eyre::WrapErr;
use fluence_libp2p::types::BackPressuredInlet;
use fluence_libp2p::RandomPeerId;
use futures::channel::mpsc;
use futures::future::BoxFuture;
use futures::{FutureExt, SinkExt};
use humantime_serde::re::humantime::format_duration as pretty;
use kademlia::KademliaApi;
use libp2p::PeerId;
use particle_closures::{HostClosures, NodeInfo};
use particle_node::{ConnectionPoolCommand, Connectivity, KademliaCommand, NetworkApi};
use particle_protocol::{Contact, Particle};
use script_storage::ScriptStorageApi;
use server_config::ServicesConfig;
use std::convert::Infallible;
use std::mem;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Waker;
use std::time::{Duration, Instant};
use test_utils::{make_tmp_dir, now_ms, put_aquamarine};

const TIMEOUT: Duration = Duration::from_secs(10);
const PARALLELISM: Option<usize> = Some(16);

async fn particles(n: usize) -> BackPressuredInlet<Particle> {
    let (mut outlet, inlet) = mpsc::channel(n * 2);

    let last_particle = std::iter::once({
        let mut p = Particle::default();
        p.id = String::from("last");
        Ok(p)
    });
    fn particle(n: usize) -> Particle {
        Particle {
            timestamp: now_ms() as u64,
            ttl: 10000,
            id: n.to_string(),
            script: String::from(r#"(call %init_peer_id% ("op" "identity") ["hello"] result)"#),
            ..<_>::default()
        }
    }
    let mut particles = futures::stream::iter((0..n).map(|i| Ok(particle(i))).chain(last_particle));
    outlet.send_all(&mut particles).await.unwrap();
    mem::forget(outlet);

    inlet
}

fn kademlia_api() -> (KademliaApi, JoinHandle<()>) {
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

fn connection_pool_api(num_particles: usize) -> (ConnectionPoolApi, JoinHandle<()>) {
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
                    out.send(Some(Contact::new(peer_id, vec![]))).unwrap()
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

fn aquamarine_api() -> (AquamarineApi, JoinHandle<()>) {
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

fn aquamarine_with_backend(
    pool_size: usize,
    delay: Option<Duration>,
) -> (AquamarineApi, JoinHandle<()>) {
    struct EasyVM {
        delay: Option<Duration>,
    }

    impl AquaRuntime for EasyVM {
        type Config = Option<Duration>;
        type Error = Infallible;

        fn create_runtime(
            delay: Option<Duration>,
            _: Waker,
        ) -> BoxFuture<'static, Result<Self, Self::Error>> {
            futures::future::ok(EasyVM { delay }).boxed()
        }

        fn into_effects(_: Result<InterpreterOutcome, Self::Error>, p: Particle) -> StepperEffects {
            StepperEffects {
                particles: vec![SendParticle {
                    target: p.init_peer_id,
                    particle: p,
                }],
            }
        }

        fn call(
            &mut self,
            init_user_id: PeerId,
            _aqua: String,
            data: Vec<u8>,
            _particle_id: String,
        ) -> Result<InterpreterOutcome, Self::Error> {
            if let Some(delay) = self.delay {
                std::thread::sleep(delay);
            }

            Ok(InterpreterOutcome {
                ret_code: 0,
                error_message: "".to_string(),
                data: data.into(),
                next_peer_pks: vec![init_user_id.to_string()],
            })
        }
    }

    let config = VmPoolConfig {
        pool_size,
        execution_timeout: TIMEOUT,
    };
    let (backend, api): (AquamarineBackend<EasyVM>, _) = AquamarineBackend::new(config, delay);
    let handle = backend.start();

    (api, handle)
}

fn aquamarine_with_vm<C>(
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
    };
    let script_storage_api = ScriptStorageApi {
        outlet: mpsc::unbounded().0,
    };
    let services_config = ServicesConfig::new(
        local_peer_id,
        tmp_dir.join("services"),
        <_>::default(),
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
    let (stepper_pool, stepper_pool_api): (AquamarineBackend<AquamarineVM>, _) =
        AquamarineBackend::new(pool_config, (vm_config, host_closures.descriptor()));

    let handle = stepper_pool.start();

    (stepper_pool_api, handle)
}

#[allow(dead_code)]
async fn network_api(particles_num: usize) -> (NetworkApi, Vec<JoinHandle<()>>) {
    let particle_stream: BackPressuredInlet<Particle> = particles(particles_num).await;
    let particle_parallelism: usize = 1;
    let (kademlia, kad_handle) = kademlia_api();
    let (connection_pool, cp_handle) = connection_pool_api(1000);
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

fn connectivity(num_particles: usize) -> (Connectivity, BoxFuture<'static, ()>, JoinHandle<()>) {
    let (kademlia, kad_handle) = kademlia_api();
    let (connection_pool, cp_handle) = connection_pool_api(num_particles);
    let connectivity = Connectivity {
        kademlia,
        connection_pool,
    };

    (connectivity, cp_handle.boxed(), kad_handle)
}

async fn process_particles(
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

fn thousand_particles_bench(c: &mut Criterion) {
    c.bench_function("thousand_particles", move |b| {
        let n = 1000;
        let particle_timeout = TIMEOUT;
        let parallelism = PARALLELISM;

        b.to_async(AsyncStdExecutor)
            .iter(move || process_particles(n, parallelism, particle_timeout))
    });
}

fn particle_throughput_bench(c: &mut Criterion) {
    let parallelism = PARALLELISM;
    let mut group = c.benchmark_group("particle_throughput");
    for size in [1, 1000, 2 * 1000, 4 * 1000, 8 * 1000, 16 * 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &n| {
            b.to_async(AsyncStdExecutor)
                .iter(move || process_particles(n, parallelism, TIMEOUT))
        });
    }
}

async fn process_particles_with_delay(
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

async fn process_particles_with_vm(
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

fn thousand_particles_with_aquamarine_bench(c: &mut Criterion) {
    c.bench_function("thousand_particles_with_aquamarine", move |b| {
        let n = 1000;
        let pool_size = 1;
        let call_time = Some(Duration::from_nanos(1));
        let particle_parallelism = PARALLELISM;
        let particle_timeout = TIMEOUT;

        b.to_async(AsyncStdExecutor).iter(move || {
            process_particles_with_delay(
                n,
                pool_size,
                call_time,
                particle_parallelism,
                particle_timeout,
            )
        })
    });
}

fn particle_throughput_with_delay_bench(c: &mut Criterion) {
    let particle_parallelism = PARALLELISM;
    let particle_timeout = TIMEOUT;

    let mut group = c.benchmark_group("particle_throughput_with_delay");
    for &num in [1, 1000, 4 * 1000, 8 * 1000].iter() {
        for delay in [None, Some(Duration::from_millis(1))].iter() {
            for &pool_size in [1, 2, 4, 16].iter() {
                group.throughput(Throughput::Elements(num as u64));
                let bid = {
                    let delay = delay.unwrap_or(Duration::from_nanos(0));
                    BenchmarkId::from_parameter(format!("{}:{}@{}", num, pretty(delay), pool_size))
                };
                group.bench_with_input(bid, &(delay, num), |b, (&delay, n)| {
                    b.to_async(AsyncStdExecutor).iter(move || {
                        process_particles_with_delay(
                            *n,
                            pool_size,
                            delay,
                            particle_parallelism,
                            particle_timeout,
                        )
                    })
                });
            }
        }
    }
}

fn particle_throughput_with_vm_bench(c: &mut Criterion) {
    let particle_parallelism = PARALLELISM;
    let particle_timeout = TIMEOUT;

    let tmp_dir = make_tmp_dir();
    let interpreter = put_aquamarine(tmp_dir.join("modules"));

    let mut group = c.benchmark_group("particle_throughput_with_vm");
    let num = 100;
    let pool_size = 4;

    // for &num in [1, 1000, 4 * 1000, 8 * 1000].iter() {
    //     for &pool_size in [1, 2, 4, 16].iter() {
    group.throughput(Throughput::Elements(num as u64));
    group.sample_size(10);
    let bid = { BenchmarkId::from_parameter(format!("{}@{}", num, pool_size)) };
    group.bench_with_input(bid, &num, |b, &n| {
        let interpreter = interpreter.clone();
        b.iter_batched(
            || {
                let interpreter = interpreter.clone();
                let peer_id = RandomPeerId::random();

                let (con, finish_fut, kademlia) = connectivity(n);
                let (aquamarine, aqua_handle) =
                    aquamarine_with_vm(pool_size, con.clone(), peer_id, interpreter.clone());
                // let (aquamarine, aqua_handle) = aquamarine_with_backend(pool_size, None);
                // dbg!(std::mem::size_of_val(&aquamarine));
                // dbg!(std::mem::size_of_val(&aqua_handle));

                let (sink, _) = mpsc::unbounded();
                let particle_stream: BackPressuredInlet<Particle> = task::block_on(particles(n));
                // dbg!(std::mem::size_of_val(&con));
                // dbg!(std::mem::size_of_val(&sink));
                // dbg!(std::mem::size_of_val(&particle_stream));
                let process_fut = Box::new(con.clone().process_particles(
                    particle_parallelism,
                    particle_stream,
                    aquamarine,
                    sink,
                    particle_timeout,
                ));

                // dbg!(std::mem::size_of_val(&finish_fut));
                // dbg!(std::mem::size_of_val(&process_fut));
                // dbg!(std::mem::size_of_val(&kademlia));

                let res = (process_fut.boxed(), finish_fut, vec![kademlia, aqua_handle]);

                res
            },
            move |(process, finish, mut handles)| {
                task::block_on(async move {
                    let start = Instant::now();
                    let process = async_std::task::spawn(process);
                    handles.push(process);
                    let spawn_took = start.elapsed().as_millis();

                    let start = Instant::now();
                    finish.await;
                    let finish_took = start.elapsed().as_millis();

                    let start = Instant::now();
                    for handle in handles {
                        handle.cancel().await;
                    }
                    let cancel_took = start.elapsed().as_millis();

                    println!(
                        "spawn {} ms; finish {} ms; cancel {} ms;",
                        spawn_took, finish_took, cancel_took
                    )
                })
            },
            BatchSize::LargeInput,
        )
    });
    //     }
    // }
}

criterion_group!(
    benches,
    thousand_particles_bench,
    particle_throughput_bench,
    thousand_particles_with_aquamarine_bench,
    particle_throughput_with_delay_bench,
    particle_throughput_with_vm_bench
);
criterion_main!(benches);
