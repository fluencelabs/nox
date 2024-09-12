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
use libp2p::core::Multiaddr;
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct NodeInfo {
    pub external_addresses: Vec<Multiaddr>,
    pub node_version: &'static str,
    pub air_version: &'static str,
    pub spell_version: String,
    pub allowed_effectors: Vec<String>,
    pub vm_info: Option<VmInfo>,
}

#[derive(Serialize, Clone, Debug)]
pub struct VmInfo {
    // Public IP via which we can connect to the VM
    pub ip: String,
    // List of ports that are forwarded to the VM
    pub forwarded_ports: Vec<PortInfo>,
    // Default SSH port to which to connect
    pub default_ssh_port: u16,
}

#[derive(Clone, Debug)]
pub enum PortInfo {
    Port(u16),
    Range(u16, u16),
}

impl Serialize for PortInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PortInfo::Port(port) => serializer.serialize_u16(*port),
            PortInfo::Range(start, end) => serializer.serialize_str(&format!("{}-{}", start, end)),
        }
    }
}
