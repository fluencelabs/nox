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
    Some(IValue::Record(NEVec::new(vec![IValue::U32(0), value]).unwrap()))
}

/// Converts erroneous result of call_service into `IValue::Record`
pub fn error(err: JValue) -> Option<IValue> {
    let err = IValue::String(err.to_string());
    Some(IValue::Record(NEVec::new(vec![IValue::U32(1), err]).unwrap()))
}

/// Converts empty result of call_service into `IValue::Record`
pub fn unit() -> Option<IValue> {
    Some(IValue::Record(
        NEVec::new(vec![IValue::S32(0), IValue::String("\"\"".to_string())]).unwrap(),
    ))
}
