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

use fluence_app_service::AppServiceError;
use particle_modules::ModuleError;

use std::{error::Error, path::PathBuf};

#[derive(Debug)]
pub enum ServiceError {
    NoSuchInstance(String),
    Engine(AppServiceError),
    ModuleError(ModuleError),
    #[allow(dead_code)]
    ReadPersistedService {
        path: PathBuf,
        err: std::io::Error,
    },
    #[allow(dead_code)]
    CreateServicesDir {
        path: PathBuf,
        err: std::io::Error,
    },
    ArgParseError(ArgsError),
    MissingBlueprintId,
}

impl Error for ServiceError {}
impl From<AppServiceError> for ServiceError {
    fn from(err: AppServiceError) -> Self {
        ServiceError::Engine(err)
    }
}

impl From<ModuleError> for ServiceError {
    fn from(err: ModuleError) -> Self {
        ServiceError::ModuleError(err)
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
            ServiceError::MissingBlueprintId => {
                write!(f, "No blueprint_id in arguments from stepper")
            }
            ServiceError::ArgParseError { field, error } => write!(
                f,
                "Error parsing arguments on call_service. field: {}, error: {:?}",
                field, error
            ),
            ServiceError::ModuleError(err) => write!(f, "ModuleError: {:?}", err),
        }
    }
}
