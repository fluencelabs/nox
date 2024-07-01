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

#![feature(try_blocks)]
#![feature(slice_as_chunks)]

mod chain_data;
mod data_tokens;
mod error;
mod log;
mod utils;

pub use chain_data::{parse_chain_data, ChainData, ChainEvent, EventField};
pub use data_tokens::{next, next_opt};
pub use error::ChainDataError;
pub use log::{parse_log, Log, LogParseError};
pub use utils::{parse_peer_id, peer_id_from_hex, peer_id_to_bytes, peer_id_to_hex};
