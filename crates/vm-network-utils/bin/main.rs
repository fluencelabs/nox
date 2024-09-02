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

use std::net::Ipv4Addr;
use std::str::FromStr;
use vm_network_utils::{clear_network, NetworkSettings};

// bin for local tests
fn main() {
    let ns = NetworkSettings {
        public_ip: Ipv4Addr::from_str("1.1.1.1").unwrap(),
        vm_ip: Ipv4Addr::from_str("2.2.2.2").unwrap(),
        bridge_name: "br0".to_string(),
        port_range: (1000, 65535),
        host_ssh_port: 2222,
        vm_ssh_port: 22,
    };
    //let result = setup_network(&ns, "test");
    let result = clear_network(&ns, "12D3KooWAb7dquiiyrxZEchx");
    println!("{result:?}");
}
