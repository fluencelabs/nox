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

use std::fmt::Debug;
use std::path::PathBuf;

use fluence_app_service::AppServiceError;
use serde_json::Value as JValue;
use thiserror::Error;

use fluence_libp2p::PeerId;
use json_utils::err_as_value;
use particle_args::ArgsError;
use particle_execution::VaultError;
use particle_modules::ModuleError;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Service with id '{0}' not found")]
    NoSuchService(String),
    #[error("Service with id '{service}' not found (function {function})")]
    NoSuchServiceWithFunction { service: String, function: String },
    #[error("Service with id '{service_id}' is deployed on another worker {worker_id})")]
    CallServiceFailedWrongWorker {
        service_id: String,
        worker_id: PeerId,
    },
    #[error("Service with alias '{0}' is not found on worker '{1}'")]
    NoSuchAlias(String, PeerId),
    #[error("Forbidden. User id '{user}' cannot call function '{function}': {reason}")]
    Forbidden {
        user: PeerId,
        function: &'static str,
        reason: &'static str,
    },
    #[error("Forbidden. User id '{0}' cannot call function 'add_alias': only management peer id can add top-level aliases")]
    ForbiddenAliasRoot(PeerId),
    #[error("Forbidden. User id '{0}' cannot call function 'add_alias': only worker and management peer id can add worker-level aliases")]
    ForbiddenAliasWorker(PeerId),
    #[error("Cannot add alias '{0}' because there is a service with that id")]
    AliasAsServiceId(String),
    #[error("Cannot add alias '{0}' because it is reserved")]
    ForbiddenAlias(String),
    #[error(
        "Alias cannot be added for service {service_id} deployed on another worker {worker_id}"
    )]
    AliasWrongWorkerId {
        service_id: String,
        worker_id: PeerId,
    },
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
    #[error("Error serializing persisted service config to toml: {err} {config:?}")]
    SerializePersistedService {
        #[source]
        err: toml::ser::Error,
        config: Box<dyn Debug + Send + Sync>,
    },
    #[error("Error saving persisted service to {path:?}: {err}")]
    WritePersistedService {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
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
