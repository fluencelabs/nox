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

use std::fmt::{Display, Formatter};

use libp2p::{core::Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

use types::peer_id;

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Contact {
    #[serde(
        serialize_with = "peer_id::serde::serialize",
        deserialize_with = "peer_id::serde::deserialize"
    )]
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
}

impl Contact {
    pub fn new(peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        Self { peer_id, addresses }
    }
}

impl Display for Contact {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.addresses.is_empty() {
            write!(f, "{} @ [no addr]", self.peer_id)
        } else {
            write!(
                f,
                "{} @ [{}, ({} more)]",
                self.peer_id,
                self.addresses[0],
                self.addresses.len() - 1
            )
        }
    }
}
