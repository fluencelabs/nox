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

#[derive(Copy, Debug, Clone)]
pub struct ListenConfig {
    pub listen_ip: IpAddr,
    pub tcp_port: u16,
    pub websocket_port: u16,
}

impl ListenConfig {
    pub fn multiaddrs(&self) -> Vec<Multiaddr> {
        let mut tcp = Multiaddr::from(self.listen_ip);
        tcp.push(Protocol::Tcp(self.tcp_port));

        let mut ws = Multiaddr::from(self.listen_ip);
        ws.push(Protocol::Tcp(self.websocket_port));
        ws.push(Protocol::Ws("/".into()));

        vec![tcp, ws]
    }
}

impl From<&ListenConfig> for Vec<Multiaddr> {
    fn from(cfg: &ListenConfig) -> Self {
        cfg.multiaddrs()
    }
}
