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

use serde_json::Value as JValue;

pub mod base64_serde;

pub fn into_string(v: JValue) -> Option<String> {
    if let JValue::String(s) = v {
        return Some(s);
    }

    None
}

pub fn into_array(v: JValue) -> Option<Vec<JValue>> {
    if let JValue::Array(s) = v {
        return Some(s);
    }

    None
}

/// Converts an error into IValue::String
pub fn err_as_value<E: core::fmt::Debug + core::fmt::Display>(err: E) -> JValue {
    JValue::String(format!("Error: {err}\n{err:?}"))
}
