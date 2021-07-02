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

use libp2p::core::Multiaddr;
use libp2p::multiaddr::Protocol;

use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct ListenConfig {
    pub multiaddrs: Vec<Multiaddr>,
}

impl ListenConfig {
    pub fn new(listen_ip: IpAddr, tcp_port: u16, websocket_port: u16) -> Self {
        let mut tcp = Multiaddr::from(listen_ip);
        tcp.push(Protocol::Tcp(tcp_port));

        let mut ws = Multiaddr::from(listen_ip);
        ws.push(Protocol::Tcp(websocket_port));
        ws.push(Protocol::Ws("/".into()));

        Self {
            multiaddrs: vec![tcp, ws],
        }
    }
}
