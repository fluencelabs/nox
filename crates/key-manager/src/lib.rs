#![feature(try_blocks)]

pub type DealId = String;

pub type WorkerId = PeerId;

mod error;
mod key_storage;
mod persistence;
mod scope;
mod worker_registry;

pub use error::KeyManagerError;
use fluence_libp2p::PeerId;
pub use key_storage::KeyStorage;
pub use scope::ScopeHelper;
pub use worker_registry::WorkerRegistry;
