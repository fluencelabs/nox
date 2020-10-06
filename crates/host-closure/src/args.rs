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

use crate::args_error::ArgsError::{self, MissingField, SerdeJson};

use control_macro::ok_get;
use ivalue_utils::{into_str, into_string, IValue};

use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(Debug)]
pub struct Args {
    pub service_id: String,
    pub fname: String,
    pub args: Vec<serde_json::Value>,
}

impl Args {
    /// Retrieves next json value from iterator, parse it to T
    pub fn next<T: DeserializeOwned>(
        field: &'static str,
        args: &mut impl Iterator<Item = Value>,
    ) -> Result<T, ArgsError> {
        let value = args.next().ok_or(MissingField(field))?;
        let value: T = Self::deserialize(field, value)?;

        Ok(value)
    }

    pub fn maybe_next<T: DeserializeOwned>(
        field: &'static str,
        args: &mut impl Iterator<Item = Value>,
    ) -> Result<Option<T>, ArgsError> {
        let value = ok_get!(args.next());
        let value: T = Self::deserialize(field, value)?;

        Ok(Some(value))
    }

    fn deserialize<T: DeserializeOwned>(field: &'static str, v: Value) -> Result<T, ArgsError> {
        serde_json::from_value(v).map_err(|err| SerdeJson { err, field })
    }

    pub fn parse(args: Vec<IValue>) -> Result<Args, ArgsError> {
        let mut args = args.into_iter();
        let service_id = args
            .next()
            .and_then(into_string)
            .ok_or(MissingField("service_id"))?;

        let fname = args
            .next()
            .and_then(into_string)
            .ok_or(MissingField("fname"))?;

        let args = args
            .next()
            .as_ref()
            .and_then(into_str)
            .ok_or(MissingField("service_id"))
            .and_then(|v| {
                println!("args: {}", v);
                serde_json::from_str(v).map_err(|err| SerdeJson {
                    field: "service_id",
                    err,
                })
            })?;
        /*        let args = match args {
            Value::Array(arr) => arr.into_iter().next().ok_or(InvalidFormat {
                field: "args",
                err: "Expected single-element array, got empty array".into(),
            }),
            o @ Value::Object(_) => Ok(o),
            v => Err(InvalidFormat {
                field: "args",
                err: format!("Expected json object, got {:?}", v).into(),
            }),
        }?;*/

        Ok(Args {
            service_id,
            fname,
            args,
        })
    }
}
