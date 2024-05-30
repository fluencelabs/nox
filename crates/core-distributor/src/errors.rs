/*
 * Copyright 2024 Fluence DAO
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

use std::fmt::{Display, Formatter, Write};
use std::str::Utf8Error;

use ccp_shared::types::CUID;
use cpu_utils::{CPUTopologyError, PhysicalCoreId};
use thiserror::Error;

use crate::types::AcquireRequest;

#[derive(Debug, Error)]
pub enum CreateError {
    #[error("System core count should be > 0")]
    IllegalSystemCoreCount,
    #[error("Too much system cores needed. Required: {required}, available: {required}")]
    NotEnoughCores { available: usize, required: usize },
    #[error("Failed to create CPU topology {err}")]
    CreateTopology { err: CPUTopologyError },
    #[error("Failed to collect cores data from OS {err:?}")]
    CollectCoresData { err: CPUTopologyError },
    #[error("The specified CPU range exceeds the available CPU count")]
    WrongCpuRange,
}

#[derive(Debug, Error)]
pub enum LoadingError {
    #[error(transparent)]
    CreateCoreDistributor {
        #[from]
        err: CreateError,
    },
    #[error("Failed to read core state: {err}")]
    IoError {
        #[from]
        err: std::io::Error,
    },
    #[error("Failed to decode core state: {err}")]
    DecodeError {
        #[from]
        err: Utf8Error,
    },
    #[error("Failed to deserialize core state: {err}")]
    DeserializationError {
        #[from]
        err: toml::de::Error,
    },
    #[error(transparent)]
    PersistError {
        #[from]
        err: PersistError,
    },
}

#[derive(Debug, Error)]
pub enum PersistError {
    #[error("Failed to persist core state: {err}")]
    IoError {
        #[from]
        err: std::io::Error,
    },
    #[error("Failed to serialize core state: {err}")]
    SerializationError {
        #[from]
        err: toml::ser::Error,
    },
}

#[derive(Debug, PartialEq)]
pub struct CurrentAssignment {
    data: Vec<(PhysicalCoreId, CUID)>,
}

impl CurrentAssignment {
    pub fn new(data: Vec<(PhysicalCoreId, CUID)>) -> Self {
        Self { data }
    }
}

impl Display for CurrentAssignment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;
        for (core, cuid) in &self.data[0..self.data.len() - 1] {
            f.write_str(core.to_string().as_str())?;
            f.write_str(" -> ")?;
            f.write_str(format!("{}", cuid).as_str())?;
            f.write_str(", ")?;
        }
        let (core, cuid) = &self.data[self.data.len() - 1];
        f.write_str(core.to_string().as_str())?;
        f.write_str(" -> ")?;
        f.write_str(format!("{}", cuid).as_str())?;

        f.write_char(']')?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum AcquireError {
    #[error("Couldn't assign core: no free cores left. Required: {required}, available: {available}, acquire_request: {acquire_request}, current assignment: {current_assignment}")]
    NotFoundAvailableCores {
        required: usize,
        available: usize,
        acquire_request: AcquireRequest,
        current_assignment: CurrentAssignment,
    },
}
