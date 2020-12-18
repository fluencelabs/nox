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

use aquamarine_vm::AquamarineVMError;
use stepper_interface::StepperOutcome;

use libp2p::PeerId;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug)]
pub enum FieldError {
    InvalidJson(serde_json::Error),
    InvalidPeerId(String),
}

impl Error for FieldError {}
impl Display for FieldError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldError::InvalidJson(err) => write!(f, "invalid json: {}", err),
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
    AquamarineError(AquamarineVMError),
}

impl Error for ExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            ExecutionError::InvalidResultField { error, .. } => Some(error),
            ExecutionError::AquamarineError(err) => Some(err),
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
        }
    }
}

fn parse_peer_id(s: &str) -> Result<PeerId, FieldError> {
    PeerId::from_str(s).map_err(|err| FieldError::InvalidPeerId(err.to_string()))
}

pub fn parse_outcome(
    outcome: Result<StepperOutcome, AquamarineVMError>,
) -> Result<(serde_json::Value, Vec<PeerId>), ExecutionError> {
    let outcome = outcome.map_err(|err| ExecutionError::AquamarineError(err))?;
    let data = serde_json::from_slice(outcome.data.as_slice()).map_err(|err| {
        ExecutionError::InvalidResultField {
            field: "data",
            error: FieldError::InvalidJson(err),
        }
    })?;
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

    Ok((data, peer_ids))
}
