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

#![feature(extract_if)]
#![feature(stmt_expr_attributes)]

mod api;
mod behaviour;
mod error;

pub use api::KademliaApi;
pub use api::KademliaApiT;
pub use behaviour::Kademlia;
pub use behaviour::KademliaConfig;
pub use error::KademliaError;

// to be available in benchmarks
pub use api::Command;
