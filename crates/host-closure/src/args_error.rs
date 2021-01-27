/*
 * Copyright 2020 Fluence Labs Limited
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

use json_utils::err_as_value;

use serde_json::Value as JValue;
use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ArgsError {
    MissingField(&'static str),
    SerdeJson {
        field: &'static str,
        err: serde_json::Error,
    },
    InvalidFormat {
        field: &'static str,
        err: Cow<'static, str>,
    },
}

impl From<ArgsError> for JValue {
    fn from(err: ArgsError) -> Self {
        err_as_value(err)
    }
}

impl Error for ArgsError {}

impl Display for ArgsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgsError::MissingField(field) => {
                write!(f, "Field {} is missing from args to call_service", field)
            }
            ArgsError::SerdeJson { err, field } => {
                write!(f, "Error while deserializing field {}: {:?}", field, err)
            }
            ArgsError::InvalidFormat { field, err } => {
                write!(f, "Error while deserializing field {}: {:?}", field, err)
            }
        }
    }
}

#[derive(Debug)]
/// An error that can be created from any other error
/// Simplifies life by converting errors to be returnable from host closures
pub struct JError(JValue);

impl From<JError> for JValue {
    fn from(err: JError) -> Self {
        err.0
    }
}

impl<E: std::error::Error> From<E> for JError {
    fn from(err: E) -> Self {
        JError(err_as_value(err))
    }
}

// It's not possible to implement Error for JError in Rust
// impl Error for JError {}
