#![feature(try_blocks)]


pub type DealId = String;

pub type WorkerId = PeerId;

mod error;
mod key_manager;
mod persistence;
mod worker_registry;
mod security;

pub use error::KeyManagerError;
use fluence_libp2p::PeerId;
pub use key_manager::KeyStorage;
