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
use types::peer_scope::{PeerScope, WorkerId};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Service with id '{0}' not found on {1:?}")]
    NoSuchService(String, PeerScope),
    #[error("Service with id '{service}' not found (function {function})")]
    NoSuchServiceWithFunction { service: String, function: String },
    #[error("Service with id '{service_id}' is deployed on another worker {peer_scope:?})")]
    CallServiceFailedWrongWorker {
        service_id: String,
        peer_scope: PeerScope,
    },
    #[error("Service with alias '{0}' is not found on worker '{1:?}'")]
    NoSuchAlias(String, PeerScope),
    #[error("Forbidden. User id '{user}' cannot call function '{function}': {reason}")]
    Forbidden {
        user: PeerId,
        function: &'static str,
        reason: &'static str,
    },
    #[error("Forbidden. User id '{0}' cannot call function 'add_alias': only management peer id can add top-level aliases")]
    ForbiddenAliasRoot(PeerId),
    #[error("Forbidden. User id '{0}' cannot call function 'add_alias': only worker, worker creator and management peer id can add worker-level aliases")]
    ForbiddenAliasWorker(PeerId),
    #[error("Cannot add alias '{0}' because there is a service with that id")]
    AliasAsServiceId(String),
    #[error("Cannot add alias '{0}' because it is reserved")]
    ForbiddenAlias(String),
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
        err: toml_edit::ser::Error,
        config: Box<dyn Debug + Send + Sync>,
    },
    #[error("Error saving persisted service to {path:?}: {err}")]
    WritePersistedService {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Internal error, smth bad happened: {0}")]
    InternalError(String),
    #[error("Worker {worker_id} not found")]
    WorkerNotFound { worker_id: WorkerId },
    #[error("Failed to create directory {path}: {err}")]
    FailedToCreateDirectory {
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
