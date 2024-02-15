#![feature(slice_take)]

extern crate core;

mod core_range;
mod manager;

pub use core_range::CoreRange;
pub use manager::CoreManager;
