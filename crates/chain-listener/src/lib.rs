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

#![feature(assert_matches)]
#![feature(try_blocks)]
#![feature(extract_if)]
#![feature(btree_extract_if)]
#![feature(result_option_inspect)]
#![feature(slice_as_chunks)]
extern crate core;

pub use event::{CommitmentActivated, ComputeUnitsMatched, UnitActivated, UnitDeactivated, CIDV1};
pub use listener::ChainListener;

mod event;
mod listener;

mod persistence;
mod proof_tracker;
mod types;
