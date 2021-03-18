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
use async_std::task::{self, spawn, JoinHandle};
use connection_pool::ConnectionPoolApi;
use criterion::{criterion_group, criterion_main};
use criterion::{BenchmarkId, Criterion};
use fluence_libp2p::types::BackPressuredInlet;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use kademlia::{Command, KademliaApi};
use particle_node::{ConnectionPoolCommand, Connectivity, KademliaCommand, NetworkApi};
use particle_protocol::Particle;
use std::mem;
use std::time::Duration;

async fn particles(n: usize) -> BackPressuredInlet<Particle> {
    let (mut outlet, inlet) = mpsc::channel(n);

    let last_particle = std::iter::once({
        let mut p = Particle::default();
        p.id = String::from("last");
        Ok(p)
    });
    let mut particles =
        futures::stream::iter((0..n).map(|_| Ok(Particle::default())).chain(last_particle));
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

fn connection_pool_api() -> (ConnectionPoolApi, JoinHandle<()>) {
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
                    if particle.id.as_str() == "last" {
                        return Poll::Ready(());
                    }
                } // TODO: mark success
                ConnectionPoolCommand::Dial { out, .. } => out.send(None).unwrap(),
                ConnectionPoolCommand::Disconnect { out, .. } => out.send(true).unwrap(),
                ConnectionPoolCommand::IsConnected { out, .. } => out.send(true).unwrap(),
                ConnectionPoolCommand::GetContact { out, .. } => out.send(None).unwrap(), // TODO: return contact
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

async fn network_api(particles_num: usize) -> (NetworkApi, JoinHandle<()>) {
    let particle_stream: BackPressuredInlet<Particle> = particles(particles_num).await;
    let particle_parallelism: usize = 1;
    let kademlia: KademliaApi = kademlia_api();
    let (connection_pool, future) = connection_pool_api();
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

fn connectivity() -> (Connectivity, JoinHandle<()>) {
    let kademlia: KademliaApi = kademlia_api();
    let (connection_pool, future) = connection_pool_api();
    let connectivity = Connectivity {
        kademlia,
        connection_pool,
    };

    (connectivity, future)
}

fn thousand_particles(c: &mut Criterion) {
    c.bench_function("thousand", move |b| {
        use criterion::async_executor::AsyncStdExecutor;

        let aquamarine = aquamarine_api();
        let (sink, _) = mpsc::unbounded();
        let particle_timeout = Duration::from_secs(1);

        b.to_async(AsyncStdExecutor).iter(move || {
            let (con, future) = connectivity();
            let aquamarine = aquamarine.clone();
            let sink = sink.clone();
            async move {
                let particle_stream: BackPressuredInlet<Particle> = particles(1000).await;
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

criterion_group!(benches, thousand_particles);
criterion_main!(benches);
