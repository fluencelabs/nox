#![feature(try_blocks)]
#![feature(extend_one)]
pub use sorcerer::Sorcerer;
pub use spell_builtins::{get_spell_info, install_spell, remove_spell, SpellInfo};

#[macro_use]
extern crate fstrings;

mod error;
mod script_executor;
mod sorcerer;
mod spell_builtins;
mod utils;
mod worker_builins;
