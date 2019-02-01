/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Contains definitions for memory errors.

use core::alloc::LayoutErr;
use std::alloc::AllocErr;
use std::error::Error;
use std::fmt::{self, Display};
use std::io;

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

/// Creates `From` instance for MemError for each specified error types.
///
/// Example:
///
/// ```
/// // usage of macro:
///
/// mem_error_from![io::Error]
///
/// // code will be generated:
///
/// impl From<io::Error> for MemError {
///    fn from(err: io::Error) -> Self {
///        MemError::from_err(err)
///    }
/// }
///
/// ```
// TODO: move this macro to utils or use 'quick-error' or 'error-chain' crates
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

mem_error_from! [LayoutErr; AllocErr; io::Error];
