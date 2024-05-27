/*
 * Copyright 2024 Fluence DAO
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
