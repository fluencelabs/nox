#![feature(slice_take)]

extern crate core;

mod core_range;
pub mod manager;

pub use core_range::CoreRange;
pub use types::unit_id::UnitId;
