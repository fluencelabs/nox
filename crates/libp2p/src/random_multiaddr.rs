/*
 * Copyright 2021 Fluence Labs Limited
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

use libp2p::core::multiaddr::Protocol;
use libp2p::core::Multiaddr;
use rand::Rng;

pub fn create_memory_maddr() -> Multiaddr {
    let port = 1 + rand::random::<u64>();
    let addr: Multiaddr = Protocol::Memory(port).into();
    addr
}

pub fn create_tcp_maddr() -> Multiaddr {
    let port: u16 = 1000 + rand::thread_rng().gen_range(1..3000);
    let mut maddr: Multiaddr = Protocol::Ip4("127.0.0.1".parse().unwrap()).into();
    maddr.push(Protocol::Tcp(port));
    maddr
}
