#![feature(assert_matches)]
#![feature(result_flattening)]
#![feature(try_blocks)]

mod connector;
mod error;
mod function;

pub use connector::CCInitParams;
pub use connector::ChainConnector;
pub use error::ConnectorError;
pub use function::*;
