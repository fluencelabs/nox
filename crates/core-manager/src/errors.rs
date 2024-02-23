use ccp_shared::types::CUID;
use cpu_utils::{CPUTopologyError, PhysicalCoreId};
use std::str::Utf8Error;
use thiserror::Error;

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
}

#[derive(Debug, Error)]
pub enum LoadingError {
    #[error(transparent)]
    CreateCoreManager {
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

#[derive(Debug, Error)]
pub enum AcquireError {
    #[error(
        "Couldn't assign core: no free cores left. Current assignment: {current_assignment:?}"
    )]
    NotFoundAvailableCores {
        current_assignment: Vec<(PhysicalCoreId, CUID)>,
    },
}
