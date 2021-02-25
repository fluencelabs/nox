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

use aquamarine_vm::{AquamarineVMError, InterpreterOutcome};

use libp2p::PeerId;
use log::LevelFilter;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug)]
pub enum FieldError {
    InvalidPeerId(String),
}

impl Error for FieldError {}
impl Display for FieldError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldError::InvalidPeerId(err) => write!(f, "invalid PeerId: {}", err),
        }
    }
}

#[derive(Debug)]
pub enum ExecutionError {
    InvalidResultField {
        field: &'static str,
        error: FieldError,
    },
    InterpreterOutcome {
        error_message: String,
        ret_code: i32,
        readable_data: String,
    },
    AquamarineError(AquamarineVMError),
}

impl Error for ExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            ExecutionError::InvalidResultField { error, .. } => Some(error),
            ExecutionError::AquamarineError(err) => Some(err),
            ExecutionError::InterpreterOutcome { .. } => None,
        }
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::InvalidResultField { field, error } => write!(
                f,
                "Execution error: invalid result field {}: {}",
                field, error
            ),
            ExecutionError::AquamarineError(err) => {
                write!(f, "Execution error: aquamarine error: {}", err)
            }
            ExecutionError::InterpreterOutcome {
                error_message,
                ret_code,
                readable_data,
            } => {
                write!(
                    f,
                    "Execution error: InterpreterOutcome (ret_code = {}): {} {}",
                    ret_code, error_message, readable_data
                )
            }
        }
    }
}

fn parse_peer_id(s: &str) -> Result<PeerId, FieldError> {
    PeerId::from_str(s).map_err(|err| FieldError::InvalidPeerId(err.to_string()))
}

pub fn parse_outcome(
    outcome: Result<InterpreterOutcome, AquamarineVMError>,
) -> Result<(Vec<u8>, Vec<PeerId>), ExecutionError> {
    let outcome = outcome.map_err(ExecutionError::AquamarineError)?;

    if outcome.ret_code != 0 {
        return Err(ExecutionError::InterpreterOutcome {
            error_message: outcome.error_message,
            ret_code: outcome.ret_code,
            readable_data: if log::max_level() > LevelFilter::Debug {
                String::from_utf8_lossy(outcome.data.as_slice()).to_string()
            } else {
                String::new()
            },
        });
    }

    let peer_ids = outcome
        .next_peer_pks
        .into_iter()
        .map(|id| {
            parse_peer_id(id.as_str()).map_err(|error| ExecutionError::InvalidResultField {
                field: "next_peer_pks[..]",
                error,
            })
        })
        .collect::<Result<_, ExecutionError>>()?;

    Ok((outcome.data, peer_ids))
}
