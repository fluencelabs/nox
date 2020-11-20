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

use serde_json::Value as JValue;
use std::{error::Error, path::PathBuf};

#[derive(Debug)]
pub enum ServiceError {
    NoSuchInstance(String),
    Engine(AppServiceError),
    ModuleError(ModuleError),
    ReadPersistedService { path: PathBuf, err: std::io::Error },
    DeserializePersistedService { err: toml::de::Error, path: PathBuf },
    CreateServicesDir { path: PathBuf, err: std::io::Error },
    CorruptedFaaSInterface(serde_json::Error),
    ArgParseError(ArgsError),
}

impl Error for ServiceError {}
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
        JValue::String(err.to_string())
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::NoSuchInstance(service_id) => {
                write!(f, "App service {} not found", service_id)
            }
            ServiceError::Engine(err) => err.fmt(f),
            ServiceError::ReadPersistedService { path, err } => write!(
                f,
                "Error reading persisted service from {:?}: {:?}",
                path, err
            ),
            ServiceError::CreateServicesDir { path, err } => write!(
                f,
                "Error creating directory for persisted services {:?}: {:?}",
                path, err
            ),
            ServiceError::ModuleError(err) => write!(f, "ModuleError: {:?}", err),
            ServiceError::ArgParseError(error) => {
                write!(f, "Error parsing arguments on call_service: {:?}", error)
            }
            ServiceError::DeserializePersistedService { err, path } => write!(
                f,
                "Error deserializing persisted service from {:?}: {:#?}",
                path, err
            ),
            ServiceError::CorruptedFaaSInterface(err) => write!(
                f,
                "CorruptedFaaSInterface: can't serialize interface to JSON: {:#?}",
                err
            ),
        }
    }
}
