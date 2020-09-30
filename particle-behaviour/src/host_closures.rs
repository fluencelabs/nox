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

use particle_actors::HostImportDescriptor;
use particle_services::{Args, IType, IValue};

use crate::mailbox::BuiltinServices;
use particle_actors::vec1::Vec1;
use serde_json::json;
use std::sync::Arc;

type ClosureDescriptor = Arc<dyn Fn() -> HostImportDescriptor + Send + Sync + 'static>;
type Closure = Arc<dyn Fn(Args) -> Option<IValue> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct Closures {
    pub create_service: Closure,
    pub call_service: Closure,
    pub builtin: Closure,
}

impl Closures {
    pub fn descriptor(self) -> ClosureDescriptor {
        Arc::new(move || {
            let this = self.clone();
            HostImportDescriptor {
                closure: Box::new(move |args| this.clone().route(args)),
                argument_types: vec![IType::String, IType::String, IType::String],
                output_type: Some(IType::Record(0)),
                error_handler: None,
            }
        })
    }

    fn route(self, args: Vec<IValue>) -> Option<IValue> {
        let args = match Args::parse(args) {
            Ok(args) => args,
            Err(err) => {
                log::warn!("error parsing args: {:?}", err);
                return as_record(Err(IValue::String(err.to_string())));
            }
        };
        log::info!("Router args: {:?}", args);
        // route
        match args.service_id.as_str() {
            "create" => (self.create_service)(args),
            s if BuiltinServices::is_builtin(&s) => (self.builtin)(args),
            _ => (self.call_service)(args),
        }
    }
}

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
