#![feature(try_blocks)]

mod error;
mod key_manager;
mod persistence;

pub use error::KeyManagerError;
pub use key_manager::KeyManager;
pub use key_manager::INSECURE_KEYPAIR_SEED;
