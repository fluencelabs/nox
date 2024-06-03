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

mod error;
mod key_storage;
mod persistence;
mod scope;
mod workers;

pub use core_manager::CoreManager;
pub use core_manager::DummyCoreManager;
pub use core_manager::StrictCoreManager;
pub use core_manager::CUID;
pub use error::KeyStorageError;
pub use error::WorkersError;
pub use key_storage::KeyStorage;
pub use scope::PeerScopes;
pub use tokio::sync::mpsc::Receiver;
pub use types::peer_scope::WorkerId;
pub use workers::Event;
pub use workers::WorkerParams;
pub use workers::Workers;
