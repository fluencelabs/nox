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

mod tracing_utils;
mod utils;

use control_macro::measure;
use kademlia::KademliaApiT;
use std::time::Instant;
use test_utils::enable_logs;
use tracing_utils::*;
use utils::*;

#[test]
fn kademlia_resolve() {
    let network_size = 10;

    let (connectivity, _finish_fut, kademlia, peer_ids) =
        connectivity_with_real_kad(1, network_size);

    async_std::task::block_on(async move {
        let start = Instant::now();
        let peer_id = peer_ids.into_iter().skip(1).next().unwrap();
        enable_logs();
        let result = measure!(connectivity.kademlia.discover_peer(peer_id).await);
        match result {
            Ok(vec) if vec.is_empty() => println!("empty vec!"),
            Err(err) => println!("err! {}", err),
            _ => {}
        }

        measure!(kademlia.cancel().await);
        println!("finished. elapsed {} ms", start.elapsed().as_millis())
    })
}
