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

use crate::ConnectedClient;

use libp2p::core::Multiaddr;
use std::str::FromStr;

pub fn connect_real(node_count: usize) -> impl Fn(usize) -> ConnectedClient {
    let nodes = vec![
        "/ip4/134.209.186.43/tcp/7001",
        "/ip4/134.209.186.43/tcp/7002",
        "/ip4/134.209.186.43/tcp/7003",
        "/ip4/134.209.186.43/tcp/7004",
        "/ip4/134.209.186.43/tcp/7005",
        "/ip4/134.209.186.43/tcp/7770",
        "/ip4/134.209.186.43/tcp/7100",
    ]
    .into_iter()
    .map(|addr| Multiaddr::from_str(addr).expect("valid multiaddr"))
    .cycle()
    .take(node_count)
    .collect::<Vec<_>>();

    move |i| ConnectedClient::connect_to(nodes[i].clone()).expect("connect client")
}
