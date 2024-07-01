/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fmt::{Display, Formatter, Write};
use std::str::Utf8Error;

use crate::CoreRange;
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
    #[error("The specified Core range {core_range} exceeds the available CPU count")]
    WrongCoreRange { core_range: CoreRange },
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
