#![feature(try_blocks)]
pub use sorcerer::Sorcerer;
pub use sorcerer::SpellCustomService;

#[macro_use]
extern crate fstrings;

mod error;
mod script_executor;
mod sorcerer;
mod spells;
mod utils;
