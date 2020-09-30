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

use crate::error::ArgParse::{Missing, SerdeJson};
use crate::error::ServiceError;
use crate::error::ServiceError::ArgParseError;
use fluence_app_service::IValue;

#[derive(Debug)]
pub struct Args {
    pub service_id: String,
    pub fname: String,
    pub args: serde_json::Value,
}

impl Args {
    pub fn parse(args: Vec<IValue>) -> Result<Args, ServiceError> {
        let mut args = args.into_iter();
        let service_id = args.next().and_then(into_string).ok_or(ArgParseError {
            field: "service_id",
            error: Missing,
        })?;
        let fname = args.next().and_then(into_string).ok_or(ArgParseError {
            field: "fname",
            error: Missing,
        })?;
        let args = args
            .next()
            .as_ref()
            .and_then(into_str)
            .ok_or(ArgParseError {
                field: "args",
                error: Missing,
            })
            .and_then(|v| {
                serde_json::from_str(v).map_err(|err| ArgParseError {
                    field: "args",
                    error: SerdeJson(err),
                })
            })?;

        Ok(Args {
            service_id,
            fname,
            args,
        })
    }
}

fn into_str(v: &IValue) -> Option<&str> {
    if let IValue::String(s) = v {
        Some(s.as_str())
    } else {
        None
    }
}

fn into_string(v: IValue) -> Option<String> {
    if let IValue::String(s) = v {
        Some(s)
    } else {
        None
    }
}
