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

use tracing::Dispatch;
use tracing_timing::{Builder, Histogram};

// let subscriber = FmtSubscriber::builder()
//     // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
//     // will be written to stdout.
//     .with_max_level(Level::TRACE)
//     // completes the builder.
//     .finish();

// LogTracer::init_with_filter(log::LevelFilter::Error).expect("Failed to set logger");

pub fn trace(mut f: impl FnMut()) {
    let subscriber = Builder::default()
        .no_span_recursion()
        .build(|| Histogram::new_with_max(1_000_000, 2).unwrap());
    let downcaster = subscriber.downcaster();
    let dispatcher = Dispatch::new(subscriber);
    tracing::dispatcher::set_global_default(dispatcher.clone())
        .expect("setting default dispatch failed");

    // =====
    // execute

    tracing::info_span!("whole bench").in_scope(|| {
        f();
    });

    // =====

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
