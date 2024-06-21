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

#![feature(slice_take)]

extern crate core;

pub type Map<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
pub(crate) type MultiMap<K, V> = multimap::MultiMap<K, V, BuildHasherDefault<FxHasher>>;
pub(crate) type BiMap<K, V> =
    bimap::BiHashMap<K, V, BuildHasherDefault<FxHasher>, BuildHasherDefault<FxHasher>>;

pub mod errors;

pub mod types;

mod core_range;
mod distributor;
mod persistence;
mod strategy;

#[cfg(feature = "dummy")]
pub mod dummy;

pub use ccp_shared::types::CUID;
pub use core_range::CoreRange;
pub use cpu_utils::pinning::ThreadPinner;
pub use cpu_utils::LogicalCoreId;
pub use cpu_utils::PhysicalCoreId;
use fxhash::FxHasher;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

pub use distributor::CoreDistributor;
#[cfg(feature = "mockall")]
pub use distributor::MockCoreDistributor;
pub use distributor::PersistentCoreDistributor;
pub use persistence::PersistenceTask;
pub use strategy::AcquireStrategy;
