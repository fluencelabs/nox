#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns,
    unreachable_code
)]
#![feature(try_blocks)]

pub use api::{ConnectionPoolApi, ConnectionPoolInlet};
// to be available in benchmarks
pub use api::Command;
pub use behaviour::ConnectionPoolBehaviour;

pub use crate::connection_pool::ConnectionPoolT;
pub use crate::connection_pool::LifecycleEvent;

mod api;
mod behaviour;
mod connection_pool;
