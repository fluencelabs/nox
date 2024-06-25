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

use fluence_it_types::ne_vec::NEVec;
pub use fluence_it_types::IType;
pub use fluence_it_types::IValue;

use serde_json::Value as JValue;

pub fn as_str(v: &IValue) -> Option<&str> {
    if let IValue::String(s) = v {
        Some(s.as_str())
    } else {
        None
    }
}

pub fn into_string(v: IValue) -> Option<String> {
    if let IValue::String(s) = v {
        Some(s)
    } else {
        None
    }
}

/// Converts result of call_service into `IValue::Record`
pub fn into_record_opt(v: std::result::Result<Option<JValue>, JValue>) -> Option<IValue> {
    match v {
        Ok(None) => unit(),
        Ok(Some(v)) => ok(v),
        Err(e) => error(e),
    }
}

/// Converts result of call_service into `IValue::Record`
pub fn into_record(v: std::result::Result<JValue, JValue>) -> Option<IValue> {
    match v {
        Ok(v) => ok(v),
        Err(e) => error(e),
    }
}

/// Converts successful result of call_service into `IValue::Record`  
pub fn ok(value: JValue) -> Option<IValue> {
    let value = IValue::String(value.to_string());
    Some(IValue::Record(
        NEVec::new(vec![IValue::U32(0), value]).unwrap(),
    ))
}

/// Converts erroneous result of call_service into `IValue::Record`
pub fn error(err: JValue) -> Option<IValue> {
    let err = IValue::String(err.to_string());
    Some(IValue::Record(
        NEVec::new(vec![IValue::U32(1), err]).unwrap(),
    ))
}

/// Converts empty result of call_service into `IValue::Record`
pub fn unit() -> Option<IValue> {
    Some(IValue::Record(
        NEVec::new(vec![IValue::S32(0), IValue::String("\"\"".to_string())]).unwrap(),
    ))
}
