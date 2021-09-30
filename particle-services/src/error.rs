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

use std::path::PathBuf;

use fluence_app_service::AppServiceError;
use serde_json::Value as JValue;
use thiserror::Error;

use aquamarine::VaultError;
use fluence_libp2p::PeerId;
use host_closure::ArgsError;
use json_utils::err_as_value;
use particle_modules::ModuleError;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Service with id '{0}' not found")]
    NoSuchService(String),
    #[error("Service with id '{service}' not found (function {function})")]
    NoSuchServiceWithFunction { service: String, function: String },
    #[error("Service with alias '{0}' not found")]
    NoSuchAlias(String),
    #[error("Forbidden. User id '{user}' cannot call function '{function}': {reason}")]
    Forbidden {
        user: PeerId,
        function: &'static str,
        reason: &'static str,
    },
    #[error("Cannot add alias '{0}' because there is a service with that id")]
    AliasAsServiceId(String),
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
    #[error("Vault linking (service {particle_id}, particle {service_id}) failed: {err:?}")]
    VaultLink {
        #[source]
        err: std::io::Error,
        particle_id: String,
        service_id: String,
    },
    #[error(transparent)]
    VaultError(#[from] VaultError),
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
