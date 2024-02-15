#![feature(try_blocks)]

pub type DealId = String;

mod error;
mod key_storage;
mod persistence;
mod scope;
mod workers;

pub use core_manager::manager::CoreManager;
pub use core_manager::manager::DefaultCoreManager;
pub use core_manager::manager::DummyCoreManager;
pub use core_manager::manager::UnitId;
pub use error::KeyStorageError;
pub use error::WorkersError;
pub use key_storage::KeyStorage;
pub use scope::PeerScopes;
pub use types::peer_scope::WorkerId;
pub use workers::WorkerParams;
pub use workers::Workers;
