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

mod utils;

use utils::*;

use async_std::task;
use async_std::task::{spawn, JoinHandle};
use criterion::async_executor::AsyncStdExecutor;
use criterion::{criterion_group, criterion_main, BatchSize};
use criterion::{BenchmarkId, Criterion, Throughput};
use fluence_libp2p::types::BackPressuredInlet;
use fluence_libp2p::RandomPeerId;
use futures::channel::mpsc;
use futures::FutureExt;
use particle_protocol::Particle;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use test_utils::{make_tmp_dir, put_aquamarine};

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

fn particle_throughput_with_kad_bench(c: &mut Criterion) {
    use tracing::Dispatch;
    use tracing_timing::{Builder, Histogram};

    // let subscriber = FmtSubscriber::builder()
    //     // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
    //     // will be written to stdout.
    //     .with_max_level(Level::TRACE)
    //     // completes the builder.
    //     .finish();

    // LogTracer::init_with_filter(log::LevelFilter::Error).expect("Failed to set logger");

    let subscriber = Builder::default()
        .no_span_recursion()
        .build(|| Histogram::new_with_max(1_000_000, 2).unwrap());
    let downcaster = subscriber.downcaster();
    let dispatcher = Dispatch::new(subscriber);
    let d2 = dispatcher.clone();
    tracing::dispatcher::set_global_default(dispatcher.clone())
        .expect("setting default dispatch failed");
    // tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let particle_parallelism = PARALLELISM;
    let particle_timeout = TIMEOUT;

    let tmp_dir = make_tmp_dir();
    let interpreter = put_aquamarine(tmp_dir.join("modules"));

    let mut group = c.benchmark_group("particle_throughput_with_kad");
    let num = 100;
    let pool_size = 4;
    let network_size = 10;

    tracing::info_span!("whole_bench").in_scope(|| {
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
    });

    std::thread::sleep(std::time::Duration::from_secs(15));

    let subscriber = downcaster.downcast(&dispatcher).expect("downcast failed");
    subscriber.force_synchronize();

    subscriber.with_histograms(|hs| {
        println!("histogram: {}", hs.len());

        for (span, events) in hs.iter_mut() {
            for (event, histogram) in events.iter_mut() {
                //

                println!("span {} event {}:", span, event);
                println!(
                    "mean: {:.1}µs, p50: {}µs, p90: {}µs, p99: {}µs, p999: {}µs, max: {}µs",
                    histogram.mean() / 1000.0,
                    histogram.value_at_quantile(0.5) / 1_000,
                    histogram.value_at_quantile(0.9) / 1_000,
                    histogram.value_at_quantile(0.99) / 1_000,
                    histogram.value_at_quantile(0.999) / 1_000,
                    histogram.max() / 1_000,
                );
            }
        }

        // for v in break_once(
        //     h.iter_linear(25_000).skip_while(|v| v.quantile() < 0.01),
        //     |v| v.quantile() > 0.95,
        // ) {
        //     println!(
        //         "{:4}µs | {:40} | {:4.1}th %-ile",
        //         (v.value_iterated_to() + 1) / 1_000,
        //         "*".repeat(
        //             (v.count_since_last_iteration() as f64 * 40.0 / h.len() as f64).ceil() as usize
        //         ),
        //         v.percentile(),
        //     );
        // }
    });
}

//     }
// }

criterion_group!(
    benches,
    thousand_particles_bench,
    particle_throughput_bench,
    thousand_particles_with_aquamarine_bench,
    particle_throughput_with_delay_bench,
    particle_throughput_with_kad_bench
);
criterion_main!(benches);
