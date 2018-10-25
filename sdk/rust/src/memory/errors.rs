use std::error::Error;
use core::alloc::LayoutErr;
use std::alloc::AllocErr;
use std::fmt;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct MemError(String);

impl Display for MemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "MemError({:?})", self)
    }
}

impl Error for MemError { }

impl MemError {

    fn from_str(message: &str) -> Self {
        MemError(format!("{}", message.to_string()))
    }

    fn from_err<E: Error + Display>(err: E) -> Self {
        MemError::from_str(&err.to_string())
    }
    
}

// todo write macros for that
impl From<LayoutErr> for MemError {
    fn from(err: LayoutErr) -> Self {
        MemError::from_err(err)
    }
}

impl From<AllocErr> for MemError {
    fn from(err: AllocErr) -> Self {
        MemError::from_err(err)
    }
}

impl From<std::io::Error> for MemError {
    fn from(err: std::io::Error) -> Self {
        MemError::from_err(err)
    }
}