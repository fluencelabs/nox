/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use json_utils::err_as_value;

use eyre::Report;
use serde_json::{json, Value as JValue};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArgsError {
    #[error("Field '{0}' is missing from args to call_service")]
    MissingField(&'static str),
    #[error("Error while deserializing field '{field}': {err}")]
    SerdeJson {
        field: &'static str,
        err: serde_json::Error,
    },
    #[error("Error while deserializing field '{field}': {err}")]
    InvalidFormat {
        field: &'static str,
        err: Cow<'static, str>,
    },
    #[error("Option's array must contain at most 1 element, '{field}' was of {length} elements")]
    NonUnaryOption { field: &'static str, length: usize },
    #[error("Error deserializing field '{field}': expected {expected_type}, got {value}")]
    DeserializeError {
        field: &'static str,
        value: JValue,
        expected_type: String,
    },
}

impl From<ArgsError> for JValue {
    fn from(err: ArgsError) -> Self {
        err_as_value(err)
    }
}

#[derive(Debug, Clone)]
/// An error that can be created from any other error
/// Simplifies life by converting errors to be returnable from host closures
pub struct JError(pub JValue);

impl Display for JError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl JError {
    pub fn new(msg: impl AsRef<str>) -> Self {
        Self(json!(msg.as_ref()))
    }

    pub fn from_eyre(err: Report) -> Self {
        JError(err_as_value(err))
    }
}

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
