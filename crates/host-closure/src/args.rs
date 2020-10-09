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
use ivalue_utils::{as_str, into_string, IValue};

use serde::Deserialize;
use serde_json::Value as JValue;

#[derive(Debug)]
/// Arguments passed by VM to host on call_service
pub struct Args {
    pub service_id: String,
    pub fname: String,
    pub args: Vec<serde_json::Value>,
}

impl Args {
    /// Construct Args from `Vec<IValue>`
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
            .and_then(as_str)
            .ok_or(MissingField("args"))
            .and_then(|v| {
                serde_json::from_str(v).map_err(|err| SerdeJson {
                    field: "service_id",
                    err,
                })
            })?;

        Ok(Args {
            service_id,
            fname,
            args,
        })
    }

    /// Retrieves next json value from iterator, parse it to T
    pub fn next<T: for<'de> Deserialize<'de>>(
        field: &'static str,
        args: &mut impl Iterator<Item = JValue>,
    ) -> Result<T, ArgsError> {
        let value = args.next().ok_or(MissingField(field))?;
        let value: T = Self::deserialize(field, value)?;

        Ok(value)
    }

    /// Retrieves a json value from iterator if it's not empty, and parses it to T
    pub fn maybe_next<T: for<'de> Deserialize<'de>>(
        field: &'static str,
        args: &mut impl Iterator<Item = JValue>,
    ) -> Result<Option<T>, ArgsError> {
        let value = ok_get!(args.next());
        let value: T = Self::deserialize(field, value)?;

        Ok(Some(value))
    }

    fn deserialize<T: for<'de> Deserialize<'de>>(
        field: &'static str,
        v: JValue,
    ) -> Result<T, ArgsError> {
        serde_json::from_value(v).map_err(|err| SerdeJson { err, field })
    }
}
