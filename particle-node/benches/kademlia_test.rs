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

#![allow(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod bench_network_models;
mod tracing_utils;

use bench_network_models::*;
use control_macro::measure;
use kademlia::KademliaApiT;
use std::time::{Duration, Instant};
use test_utils::enable_logs;
use tracing_utils::*;

#[test]
fn kademlia_resolve() {
    let network_size = 100;

    let (connectivity, _finish_fut, kademlia, peer_ids) =
        connectivity_with_real_kad(1, network_size);

    async_std::task::block_on(async move {
        // enable_logs();
        // log::error!("===== test before =====");
        // async_std::task::sleep(Duration::from_secs(1)).await;
        // log::error!("===== test after =====");

        let start = Instant::now();
        let peer_id = peer_ids.into_iter().skip(3).next().unwrap();
        let result = measure!(connectivity.kademlia.discover_peer(peer_id).await);
        match result {
            Ok(vec) if vec.is_empty() => println!("empty vec!"),
            Err(err) => println!("err! {}", err),
            Ok(vec) => println!("peer discovered! {:?}", vec),
        }

        measure!(kademlia.cancel().await);
        println!("finished. elapsed {} ms", start.elapsed().as_millis())
    })
}
