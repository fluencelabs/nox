use hwloc2::TypeDepthError;
use std::str::Utf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CreateError {
    #[error("System core count should be > 0")]
    IllegalSystemCoreCount,
    #[error("Too much system cores needed. Required: {required}, available: {required}")]
    NotEnoughCores { available: usize, required: usize },
    #[error("Failed to create topology")]
    CreateTopology,
    #[error("Failed to collect cores data {err:?}")]
    CollectCoresData { err: TypeDepthError },
}

#[derive(Debug, Error)]
pub enum LoadingError {
    #[error(transparent)]
    CreateCoreManager {
        #[from]
        err: CreateError,
    },
    #[error(transparent)]
    IoError {
        #[from]
        err: std::io::Error,
    },
    #[error(transparent)]
    DecodeError {
        #[from]
        err: Utf8Error,
    },
    #[error(transparent)]
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
    #[error(transparent)]
    IoError {
        #[from]
        err: std::io::Error,
    },
    #[error(transparent)]
    SerializationError {
        #[from]
        err: toml::ser::Error,
    },
}

#[derive(Debug, Error)]
pub enum AssignError {
    #[error("Not found free cores")]
    NotFoundAvailableCores,
}
