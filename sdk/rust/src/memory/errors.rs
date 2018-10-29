//! Contains defenition for memory errors.

use core::alloc::LayoutErr;
use std::alloc::AllocErr;
use std::error::Error;
use std::fmt;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct MemError(String);

impl Display for MemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "MemError({:?})", self)
    }
}

impl Error for MemError {}

impl MemError {
    pub fn new(message: &str) -> Self {
        MemError(message.to_string())
    }

    pub fn from_err<E: Error + Display>(err: &E) -> Self {
        MemError::new(&err.to_string())
    }
}

/// Creates From instance for MemError for each specified error types.
///
/// Example:
///
/// ```
/// // usage of macro:
///
/// mem_error_from![std::io::Error]
///
/// // code will be generated:
///
/// impl From<std::io::Error> for MemError {
///    fn from(err: std::io::Error) -> Self {
///        MemError::from_err(err)
///    }
/// }
///
/// ```
// todo move this macro to utils or use 'quick-error' or 'error-chain' crates
macro_rules! mem_error_from {
    ( $( $x:ty );* ) => {
        $(
            impl From<$x> for MemError {
                fn from(err: $x) -> Self {
                    MemError::from_err(&err)
                }
            }
        )*
    }
}

mem_error_from! [LayoutErr; AllocErr; std::io::Error];
