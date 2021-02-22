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

use particle_modules::ModuleError;

use fluence_app_service::AppServiceError;
use host_closure::ArgsError;
use json_utils::err_as_value;

use serde_json::Value as JValue;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("App service {0} not found")]
    NoSuchInstance(String),
    #[error("Forbidden. User id '{0}' cannot call function '{1}'")]
    Forbidden(String, String),
    #[error(transparent)]
    Engine(AppServiceError),
    #[error(transparent)]
    ModuleError(ModuleError),
    #[error("Error reading persisted service from {path:?}: {err}")]
    ReadPersistedService {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error deserializing persisted service from {path:?}: {err}")]
    DeserializePersistedService {
        path: PathBuf,
        #[source]
        err: toml::de::Error,
    },
    #[error("Error creating directory for persisted services {path:?}: {err}")]
    CreateServicesDir {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("CorruptedFaaSInterface: can't serialize interface to JSON: {0}")]
    CorruptedFaaSInterface(#[source] serde_json::Error),
    #[error("Error parsing arguments on call_service: {0}")]
    ArgParseError(#[source] ArgsError),
}

impl From<AppServiceError> for ServiceError {
    fn from(err: AppServiceError) -> Self {
        ServiceError::Engine(err)
    }
}

impl From<ArgsError> for ServiceError {
    fn from(err: ArgsError) -> Self {
        ServiceError::ArgParseError(err)
    }
}

impl From<ModuleError> for ServiceError {
    fn from(err: ModuleError) -> Self {
        ServiceError::ModuleError(err)
    }
}

impl From<ServiceError> for JValue {
    fn from(err: ServiceError) -> Self {
        err_as_value(err)
    }
}
