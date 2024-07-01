/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
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
