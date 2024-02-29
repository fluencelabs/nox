#![feature(try_blocks)]

mod error;
mod key_storage;
mod persistence;
mod scope;
mod workers;

pub use aqua_runtime::AVMRunner;
pub use aqua_runtime::AquaRuntime;
pub use aqua_runtime::VmConfig;
pub use core_manager::manager::CoreManager;
pub use core_manager::manager::DummyCoreManager;
pub use core_manager::manager::PersistentCoreManager;
pub use core_manager::CUID;
pub use error::KeyStorageError;
pub use error::WorkersError;
pub use key_storage::KeyStorage;
pub use scope::PeerScopes;
pub use types::peer_scope::WorkerId;
pub use workers::WorkerParams;
pub use workers::Workers;
pub use workers::WorkersOperations;
