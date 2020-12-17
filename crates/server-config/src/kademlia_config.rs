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

use std::time::Duration;

/// see `libp2p_kad::KademliaConfig`
pub struct KademliaConfig {
    pub max_packet_size: usize,
    pub query_timeout: Duration,
    pub replication_factor: usize,
    pub connection_idle_timeout: Duration,
}

// Defaults:
//
// cfg.set_max_packet_size(100 * 4096 * 4096) // 100 Mb
// // .set_query_timeout(Duration::from_secs(5))
// // .set_replication_factor(std::num::NonZeroUsize::new(5).unwrap())
// .set_connection_idle_timeout(Duration::from_secs(2_628_000_000)); // ~month
