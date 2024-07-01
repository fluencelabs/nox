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

mod error;
mod key_storage;
mod persistence;
mod scope;
mod workers;

pub use core_distributor::CoreDistributor;
pub use core_distributor::PersistentCoreDistributor;
pub use core_distributor::CUID;
pub use error::KeyStorageError;
pub use error::WorkersError;
pub use key_storage::KeyStorage;
pub use scope::PeerScopes;
pub use tokio::sync::mpsc::Receiver;
pub use types::peer_scope::WorkerId;
pub use workers::Event;
pub use workers::WorkerParams;
pub use workers::Workers;
