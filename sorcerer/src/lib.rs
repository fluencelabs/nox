#![feature(try_blocks)]
#![feature(extend_one)]
pub use sorcerer::Sorcerer;

#[macro_use]
extern crate fstrings;

mod error;
mod script_executor;
mod sorcerer;
mod spells;
mod utils;
mod worker_builins;
