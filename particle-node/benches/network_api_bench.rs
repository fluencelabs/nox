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

mod bench_network_models;
mod tracing_utils;

use bench_network_models::*;
use tracing_utils::*;

use async_std::task;
use async_std::task::spawn;
use connection_pool::ConnectionPoolT;
use criterion::async_executor::AsyncStdExecutor;
use criterion::{criterion_group, criterion_main, BatchSize};
use criterion::{BenchmarkId, Criterion, Throughput};
use fluence_libp2p::types::BackPressuredInlet;
use fluence_libp2p::RandomPeerId;
use futures::channel::mpsc;
use futures::FutureExt;
use humantime_serde::re::humantime::format_duration as pretty;
use kademlia::KademliaApiT;
use libp2p::PeerId;
use particle_protocol::{Contact, Particle};
use std::convert::identity;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};
use test_utils::{
    enable_logs, make_swarms_with_mocked_vm, make_tmp_dir, put_aquamarine, ConnectedClient,
};

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

fn particle_throughput_with_kad_bench(c: &mut Criterion) {
    // enable_logs();

    let particle_parallelism = PARALLELISM;
    let particle_timeout = TIMEOUT;

    let tmp_dir = make_tmp_dir();
    let interpreter = put_aquamarine(tmp_dir.join("modules"));

    let mut group = c.benchmark_group("particle_throughput_with_kad");
    let num = 100;
    let pool_size = 4;
    let network_size = 10;

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

                let (con, finish_fut, kademlia, peer_ids) =
                    connectivity_with_real_kad(n, network_size);
                // let (aquamarine, aqua_handle) =
                //     aquamarine_with_vm(pool_size, con.clone(), peer_id, interpreter.clone());
                // let (aquamarine, aqua_handle) = aquamarine_with_backend(pool_size, None);
                let (aquamarine, aqua_handle) = aquamarine_api();

                let (sink, _) = mpsc::unbounded();
                let particle_stream: BackPressuredInlet<Particle> =
                    task::block_on(particles_to_network(n, peer_ids));
                let process_fut = Box::new(con.clone().process_particles(
                    particle_parallelism,
                    particle_stream,
                    aquamarine,
                    sink,
                    particle_timeout,
                ));

                let res = (process_fut.boxed(), finish_fut, kademlia, aqua_handle);

                std::thread::sleep(Duration::from_secs(5));

                println!("finished batch setup");

                res
            },
            move |(process, finish, kad_handle, aqua_handle)| {
                task::block_on(async move {
                    println!("start iteration");
                    let start = Instant::now();
                    let process = async_std::task::spawn(process);
                    let spawn_took = start.elapsed().as_millis();

                    let start = Instant::now();
                    finish.await;
                    let finish_took = start.elapsed().as_millis();

                    let start = Instant::now();
                    kad_handle.cancel().await;
                    aqua_handle.cancel().await;
                    process.cancel().await;
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
}

fn kademlia_resolve_bench(c: &mut Criterion) {
    use control_macro::measure;

    let mut group = c.benchmark_group("kademlia_resolve");
    println!();

    let network_size = 100;
    // group.throughput(Throughput::Elements(num as u64));
    group.sample_size(10);
    let bid = { BenchmarkId::from_parameter(format!("{} nodes", network_size)) };
    group.bench_function(bid, |b| {
        b.iter_batched(
            || connectivity_with_real_kad(1, network_size),
            move |(connectivity, _finish_fut, kademlia, peer_ids)| {
                task::block_on(async move {
                    let start = Instant::now();
                    // let peer_id = peer_ids.into_iter().skip(1).next().unwrap();
                    // enable_logs();
                    for peer_id in peer_ids.into_iter().skip(1) {
                        let result = measure!(connectivity.kademlia.discover_peer(peer_id).await);
                        match result {
                            Ok(vec) if vec.is_empty() => println!("empty vec!"),
                            Err(err) => println!("err! {}", err),
                            _ => {}
                        }
                    }

                    kademlia.cancel().await;
                    println!("finished. elapsed {} ms", start.elapsed().as_millis())
                })
            },
            BatchSize::SmallInput,
        )
    });
}

fn connectivity_bench(c: &mut Criterion) {
    use control_macro::measure;

    let mut group = c.benchmark_group("connectivity");
    println!();

    let num_particles = 10;
    let network_size = 10;
    let swarms = make_swarms_with_mocked_vm(network_size, identity, None, identity);
    let first = swarms.iter().next().unwrap();
    let last = swarms.iter().last().unwrap();

    enable_logs();

    group.throughput(Throughput::Elements(num_particles as u64));
    group.sample_size(10);
    let bid = { BenchmarkId::from_parameter(format!("{}/{}", network_size, num_particles)) };
    group.bench_function(bid, |b| {
        b.iter_batched(
            || {
                let sender_node = make_swarms_with_mocked_vm(1, identity, None, identity)
                    .into_iter()
                    .next()
                    .unwrap();
                task::block_on(
                    sender_node
                        .connectivity
                        .connection_pool
                        .connect(Contact::new(first.peer_id, vec![first.multiaddr.clone()])),
                );

                let receiver_node = make_swarms_with_mocked_vm(1, identity, None, identity)
                    .into_iter()
                    .next()
                    .unwrap();

                task::block_on(
                    receiver_node
                        .connectivity
                        .connection_pool
                        .connect(Contact::new(last.peer_id, vec![last.multiaddr.clone()])),
                );

                let receiver_client = ConnectedClient::connect_to(receiver_node.multiaddr.clone())
                    .expect("connect receiver_client");

                println!(
                    "data = {},{},{}",
                    first.peer_id, receiver_node.peer_id, receiver_client.peer_id
                );
                let particles = generate_particles(num_particles, |_, mut p| {
                    p.ttl = (u16::MAX - 10) as _;
                    p.script = String::from("!");
                    p.data = format!(
                        "{},{},{}",
                        first.peer_id, receiver_node.peer_id, receiver_client.peer_id
                    )
                    .into_bytes();
                    p
                });

                (sender_node, receiver_client, particles)
            },
            move |(sender_node, mut receiver_client, particles)| {
                task::block_on(async move {
                    //
                    let contact = Contact::new(sender_node.peer_id, vec![]);
                    let num_particles = particles.len();
                    for particle in particles {
                        sender_node
                            .connectivity
                            .connection_pool
                            .send(contact.clone(), particle);
                    }
                    for _ in 0..num_particles {
                        if let Err(err) = receiver_client.receive() {
                            println!("error receiving: {:?}", err);
                        }
                    }
                })
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    thousand_particles_bench,
    particle_throughput_bench,
    thousand_particles_with_aquamarine_bench,
    particle_throughput_with_delay_bench,
    particle_throughput_with_kad_bench,
    kademlia_resolve_bench,
    connectivity_bench
);
criterion_main!(benches);
