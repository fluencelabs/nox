#![feature(try_blocks)]
mod error;
mod resolve;

pub use resolve::{resolve_subnet, SubnetResolveResult, Worker};
