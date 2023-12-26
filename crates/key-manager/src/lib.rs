#![feature(try_blocks)]

pub type DealId = String;

pub type WorkerId = PeerId;

mod error;
mod key_manager;
mod persistence;
mod security;
mod worker_registry;

pub use error::KeyManagerError;
use fluence_libp2p::PeerId;
pub use key_manager::KeyStorage;
