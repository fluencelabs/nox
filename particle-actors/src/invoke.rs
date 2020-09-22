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

use fluence_app_service::{AppServiceError, IValue};
use libp2p::PeerId;
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug)]
pub enum FieldError {
    Missing,
    InvalidType,
    InvalidJson(serde_json::Error),
    InvalidPeerId(String),
}

#[derive(Debug)]
pub enum ExecutionError {
    ResultEmpty,
    ResultIsNotARecord,
    InvalidResultField {
        field: &'static str,
        error: FieldError,
    },
    AppServiceError(AppServiceError),
}

fn parse_data(data: Option<IValue>) -> Result<Value, FieldError> {
    let data = data.ok_or(FieldError::Missing)?;
    let data = match data {
        IValue::String(s) => Ok(s),
        _ => Err(FieldError::InvalidType),
    }?;
    let data = serde_json::from_str(data.as_str()).map_err(|err| FieldError::InvalidJson(err))?;

    Ok(data)
}

fn parse_peer_id(s: &str) -> Result<PeerId, FieldError> {
    PeerId::from_str(s).map_err(|err| FieldError::InvalidPeerId(err.to_string()))
}

fn parse_next_peer_pks(pks: Option<IValue>) -> Result<Vec<PeerId>, ExecutionError> {
    let next_peer_pks = pks.ok_or(ExecutionError::InvalidResultField {
        field: "next_peer_pks",
        error: FieldError::Missing,
    })?;
    let next_peer_pks = if let IValue::Array(values) = next_peer_pks {
        values
            .into_iter()
            .map(|v| match v {
                IValue::String(s) => {
                    parse_peer_id(s.as_str()).map_err(|error| ExecutionError::InvalidResultField {
                        field: "next_peer_pks[..]",
                        error,
                    })
                }
                _ => Err(ExecutionError::InvalidResultField {
                    field: "next_peer_pks[..]",
                    error: FieldError::InvalidType,
                }),
            })
            .collect::<Result<Vec<_>, _>>()
    } else {
        Err(ExecutionError::InvalidResultField {
            field: "next_peer_pks",
            error: FieldError::InvalidType,
        })
    }?;

    Ok(next_peer_pks)
}

pub fn parse_invoke_result(
    result: Result<Vec<IValue>, AppServiceError>,
) -> Result<(serde_json::Value, Vec<PeerId>), ExecutionError> {
    let result = result.map_err(|err| ExecutionError::AppServiceError(err))?;
    let result = result
        .into_iter()
        .next()
        .ok_or(ExecutionError::ResultEmpty)?;
    match result {
        IValue::Record(fields) => {
            let mut fields = fields.into_vec().into_iter();
            let data =
                parse_data(fields.next()).map_err(|error| ExecutionError::InvalidResultField {
                    field: "data",
                    error,
                })?;
            let next_peer_pks = parse_next_peer_pks(fields.next())?;

            Ok((data, next_peer_pks))
        }
        _ => Err(ExecutionError::ResultIsNotARecord),
    }
}
