#![feature(slice_take)]

extern crate core;
mod core_range;
pub mod errors;
pub mod manager;
pub mod types;

pub use ::types::unit_id::UnitId;
pub use core_range::CoreRange;
