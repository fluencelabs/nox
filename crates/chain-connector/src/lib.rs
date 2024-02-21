#![feature(assert_matches)]

mod connector;
mod error;
mod function;

pub use connector::ChainConnector;
pub use error::ConnectorError;
pub use function::*;
