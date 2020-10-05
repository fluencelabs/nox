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

pub use wasmer_wit::types::InterfaceType as IType;
pub use wasmer_wit::values::InterfaceValue as IValue;
pub use wasmer_wit::vec1;

use serde_json::json;
use vec1::Vec1;

pub fn as_record(v: std::result::Result<IValue, IValue>) -> Option<IValue> {
    let (code, result) = match v {
        Ok(v) => (0, v),
        Err(e) => (1, e),
    };
    let result = ivalue_to_jvalue_string(result);
    let vec = Vec1::new(vec![IValue::U32(code), result]).expect("not empty");
    Some(IValue::Record(vec))
}

/// Serializes IValue to json bytes
fn ivalue_to_jvalue(v: IValue) -> serde_json::Value {
    match v {
        IValue::S8(v) => json!(v),
        IValue::S16(v) => json!(v),
        IValue::S32(v) => json!(v),
        IValue::S64(v) => json!(v),
        IValue::U8(v) => json!(v),
        IValue::U16(v) => json!(v),
        IValue::U32(v) => json!(v),
        IValue::U64(v) => json!(v),
        IValue::F32(v) => json!(v),
        IValue::F64(v) => json!(v),
        IValue::String(v) => json!(v),
        IValue::I32(v) => json!(v),
        IValue::I64(v) => json!(v),
        IValue::Array(v) => json!(v.into_iter().map(ivalue_to_jvalue).collect::<Vec<_>>()),
        IValue::Record(v) => json!(v
            .into_vec()
            .into_iter()
            .map(ivalue_to_jvalue)
            .collect::<Vec<_>>()),
    }
}

#[allow(dead_code)]
fn ivalue_to_jvalue_bytes(ivalue: IValue) -> IValue {
    let jvalue = ivalue_to_jvalue(ivalue);
    let bytes = serde_json::to_vec(&jvalue).expect("shouldn't fail");
    let bytes = bytes.into_iter().map(IValue::U8).collect();
    IValue::Array(bytes)
}

#[allow(dead_code)]
fn ivalue_to_jvalue_string(ivalue: IValue) -> IValue {
    let jvalue = ivalue_to_jvalue(ivalue);
    let string = serde_json::to_string(&jvalue).expect("shouldn't fail");
    IValue::String(string)
}
