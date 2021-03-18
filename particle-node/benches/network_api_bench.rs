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

use aquamarine::{AquamarineApi, SendParticle, StepperEffects};
use async_std::task::{spawn, JoinHandle};
use connection_pool::ConnectionPoolApi;
use criterion::async_executor::AsyncStdExecutor;
use criterion::{criterion_group, criterion_main};
use criterion::{BenchmarkId, Criterion, Throughput};
use fluence_libp2p::types::BackPressuredInlet;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use kademlia::KademliaApi;
use particle_node::{ConnectionPoolCommand, Connectivity, KademliaCommand, NetworkApi};
use particle_protocol::{Contact, Particle};
use std::mem;
use std::time::Duration;
use test_utils::now_ms;

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
    let (outlet, mut inlet) = mpsc::unbounded();
    let api = KademliaApi { outlet };

    spawn(futures::future::poll_fn::<(), _>(move |cx| {
        use std::task::Poll;

        let mut wake = false;
        while let Poll::Ready(Some(cmd)) = inlet.poll_next_unpin(cx) {
            wake = true;
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
    let num_particles = (num_particles - 1).to_string();
    let (outlet, mut inlet) = mpsc::unbounded();
    let api = ConnectionPoolApi {
        outlet,
        send_timeout: Duration::from_secs(1),
    };

    let future = spawn(futures::future::poll_fn(move |cx| {
        use std::task::Poll;

        let mut wake = false;
        while let Poll::Ready(Some(cmd)) = inlet.poll_next_unpin(cx) {
            wake = true;

            match cmd {
                ConnectionPoolCommand::Connect { out, .. } => out.send(true).unwrap(),
                ConnectionPoolCommand::Send { out, particle, .. } => {
                    out.send(true).unwrap();
                    if particle.id == num_particles {
                        return Poll::Ready(());
                    }
                } // TODO: mark success
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
    let (outlet, mut inlet) = mpsc::channel(100);

    let api = AquamarineApi::new(outlet, Duration::from_secs(1));

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

fn setup(c: &mut Criterion) {
    c.bench_function("setup", move |b| {
        let n = 1000;

        let aquamarine = aquamarine_api();
        let (sink, _) = mpsc::unbounded();
        let particle_timeout = Duration::from_secs(1);

        b.to_async(AsyncStdExecutor).iter(move || {
            let (con, _) = connectivity(n);
            let aquamarine = aquamarine.clone();
            let sink = sink.clone();
            async move {
                let particle_stream: BackPressuredInlet<Particle> = particles(n).await;
                spawn(con.clone().process_particles(
                    particle_stream,
                    aquamarine.clone(),
                    sink.clone(),
                    particle_timeout,
                ));
            }
        })
    });
}

fn thousand_particles(c: &mut Criterion) {
    c.bench_function("thousand_particles", move |b| {
        let n = 1000;

        let aquamarine = aquamarine_api();
        let (sink, _) = mpsc::unbounded();
        let particle_timeout = Duration::from_secs(1);

        b.to_async(AsyncStdExecutor).iter(move || {
            let (con, future) = connectivity(n);
            let aquamarine = aquamarine.clone();
            let sink = sink.clone();
            async move {
                let particle_stream: BackPressuredInlet<Particle> = particles(n).await;
                spawn(con.clone().process_particles(
                    particle_stream,
                    aquamarine.clone(),
                    sink.clone(),
                    particle_timeout,
                ));
                future.await
            }
        })
    });
}

fn range_particles(c: &mut Criterion) {
    let mut group = c.benchmark_group("particle_throughput");
    for size in [1000, 2 * 1000, 4 * 1000, 8 * 1000, 16 * 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &n| {
            let aquamarine = aquamarine_api();
            let (sink, _) = mpsc::unbounded();
            let particle_timeout = Duration::from_secs(1);
            b.to_async(AsyncStdExecutor).iter(|| async {
                let (con, future) = connectivity(n);
                let particle_stream: BackPressuredInlet<Particle> = particles(n).await;
                spawn(con.clone().process_particles(
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

criterion_group!(benches, setup, thousand_particles, range_particles);
criterion_main!(benches);
