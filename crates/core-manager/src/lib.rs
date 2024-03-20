#![feature(slice_take)]

extern crate core;

pub mod errors;

pub mod types;

mod core_range;

mod dev;

mod dummy;

mod manager;
mod persistence;
mod strict;

pub use ccp_shared::types::CUID;
pub use core_range::CoreRange;
pub use cpu_utils::LogicalCoreId;
pub use cpu_utils::PhysicalCoreId;
pub use dev::DevCoreManager;
pub use dummy::DummyCoreManager;
pub use manager::CoreManager;
pub use manager::CoreManagerFunctions;
pub use strict::StrictCoreManager;
