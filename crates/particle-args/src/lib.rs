#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod args;
mod args_error;
mod base58;

pub use args::Args;
pub use args_error::{ArgsError, JError};

pub use avm_server::AVMError;
pub use base58::from_base58;
