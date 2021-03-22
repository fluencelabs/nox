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
    AquaRuntime, AquamarineApi, AquamarineBackend, InterpreterOutcome, SendParticle,
    StepperEffects, VmPoolConfig,
};
use async_std::task::{spawn, JoinHandle};
use connection_pool::ConnectionPoolApi;
use criterion::async_executor::AsyncStdExecutor;
use criterion::{criterion_group, criterion_main};
use criterion::{BenchmarkId, Criterion, Throughput};
use fluence_libp2p::types::BackPressuredInlet;
use futures::channel::mpsc;
use futures::future::BoxFuture;
use futures::SinkExt;
use kademlia::KademliaApi;
use libp2p::PeerId;
use particle_node::{ConnectionPoolCommand, Connectivity, KademliaCommand, NetworkApi};
use particle_protocol::{Contact, Particle};
use std::convert::Infallible;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Waker;
use std::time::{Duration, Instant};
use test_utils::now_ms;

const TIMEOUT: Duration = Duration::from_secs(10);

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
            ..<_>::default()
        }
    }
    let mut particles = futures::stream::iter((0..n).map(|i| Ok(particle(i))).chain(last_particle));
    outlet.send_all(&mut particles).await.unwrap();
    mem::forget(outlet);

    inlet
}

fn kademlia_api() -> KademliaApi {
    use futures::StreamExt;

    let (outlet, mut inlet) = mpsc::unbounded();
    let api = KademliaApi { outlet };

    spawn(futures::future::poll_fn::<(), _>(move |cx| {
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

    api
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
                ConnectionPoolCommand::Send { out, particle, .. } => {
                    let num = counter.fetch_add(1, Ordering::Relaxed);
                    out.send(true).unwrap();
                    // println!("particle {} num {}", particle.id, num);
                    if num == num_particles - 1 {
                        return Poll::Ready(());
                    }
                }
                ConnectionPoolCommand::Dial { out, .. } => out.send(None).unwrap(),
                ConnectionPoolCommand::Disconnect { out, .. } => out.send(true).unwrap(),
                ConnectionPoolCommand::IsConnected { out, .. } => out.send(true).unwrap(),
                ConnectionPoolCommand::GetContact { peer_id, out } => {
                    out.send(Some(Contact::new(peer_id, vec![]))).unwrap()
                } // TODO: return contact
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

fn aquamarine_api() -> AquamarineApi {
    use futures::StreamExt;

    let (outlet, mut inlet) = mpsc::channel(100);

    let api = AquamarineApi::new(outlet, TIMEOUT);

    spawn(futures::future::poll_fn::<(), _>(move |cx| {
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

    api
}

fn aquamarine_with_backend(pool_size: usize, delay: Option<Duration>) -> AquamarineApi {
    use futures::FutureExt;

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
            if _particle_id.ends_with("99") {
                // println!("particle_id: {}", _particle_id);
            }
            // if _particle_id.chars().rev().next().unwrap() == '0' {
            // //     println!("particle_id: {}", _particle_id);
            // }
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
    backend.start();

    api
}

#[allow(dead_code)]
async fn network_api(particles_num: usize) -> (NetworkApi, JoinHandle<()>) {
    let particle_stream: BackPressuredInlet<Particle> = particles(particles_num).await;
    let particle_parallelism: usize = 1;
    let kademlia: KademliaApi = kademlia_api();
    let (connection_pool, future) = connection_pool_api(1000);
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
    (api, future)
}

fn connectivity(num_particles: usize) -> (Connectivity, JoinHandle<()>) {
    let kademlia: KademliaApi = kademlia_api();
    let (connection_pool, future) = connection_pool_api(num_particles);
    let connectivity = Connectivity {
        kademlia,
        connection_pool,
    };

    (connectivity, future)
}

fn thousand_particles(c: &mut Criterion) {
    c.bench_function("thousand_particles", move |b| {
        let n = 1000;

        let particle_timeout = TIMEOUT;

        b.to_async(AsyncStdExecutor).iter(move || async move {
            let (con, future) = connectivity(n);
            let aquamarine = aquamarine_api();
            let (sink, _) = mpsc::unbounded();

            let particle_stream: BackPressuredInlet<Particle> = particles(n).await;
            spawn(con.clone().process_particles(
                None,
                particle_stream,
                aquamarine,
                sink,
                particle_timeout,
            ));
            future.await
        })
    });
}

fn thousand_particles_with_aquamarine(c: &mut Criterion) {
    let start = Instant::now();

    c.bench_function("thousand_particles_with_aquamarine", move |b| {
        let n = 1000;
        let pool_size = 1;
        let call_time = Some(Duration::from_millis(1));
        let particle_parallelism = None;
        let particle_timeout = TIMEOUT;

        // println!("\n");

        b.to_async(AsyncStdExecutor).iter(move || async move {
            let (con, future) = connectivity(n);
            let aquamarine = aquamarine_with_backend(pool_size, call_time);
            let (sink, _) = mpsc::unbounded();

            let id = start.elapsed().as_millis();
            // println!("====> START bench ========{}", id);

            let particle_stream: BackPressuredInlet<Particle> = particles(n).await;
            spawn(con.clone().process_particles(
                particle_parallelism,
                particle_stream,
                aquamarine.clone(),
                sink.clone(),
                particle_timeout,
            ));
            future.await;

            // println!("====< END bench   ========{}", id)
        })
    });
}

fn particle_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("particle_throughput");
    for size in [0, 1000, 2 * 1000, 4 * 1000, 8 * 1000, 16 * 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &n| {
            let aquamarine = aquamarine_api();
            let (sink, _) = mpsc::unbounded();
            let particle_timeout = TIMEOUT;
            b.to_async(AsyncStdExecutor).iter(|| async {
                let (con, future) = connectivity(n);
                let particle_stream: BackPressuredInlet<Particle> = particles(n).await;
                spawn(con.clone().process_particles(
                    None,
                    particle_stream,
                    aquamarine.clone(),
                    sink.clone(),
                    particle_timeout,
                ));
                future.await;
            })
        });
    }
}

criterion_group!(
    benches,
    thousand_particles,
    particle_throughput,
    thousand_particles_with_aquamarine
);
criterion_main!(benches);
