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
use std::error::Error;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ServiceExecError {
    NoSuchInstance(String),
    Engine(AppServiceError),
    AddModule { path: PathBuf, err: std::io::Error },
    SerializeConfig { err: toml::ser::Error },
    WriteConfig { path: PathBuf, err: std::io::Error },
    NoSuchBlueprint { path: PathBuf, err: std::io::Error },
    IncorrectBlueprint { err: toml::de::Error },
    NoModuleConfig { path: PathBuf, err: std::io::Error },
    IncorrectModuleConfig { err: toml::de::Error },
    WriteBlueprint { path: PathBuf, err: std::io::Error },
    CreateServiceBaseDir { path: PathBuf, err: std::io::Error },
}

impl Error for ServiceExecError {}
impl From<AppServiceError> for ServiceExecError {
    fn from(err: AppServiceError) -> Self {
        ServiceExecError::Engine(err)
    }
}

impl std::fmt::Display for ServiceExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceExecError::NoSuchInstance(service_id) => {
                write!(f, "App service {} not found", service_id)
            }
            ServiceExecError::Engine(err) => err.fmt(f),
            ServiceExecError::AddModule { path, err } => {
                write!(f, "Error saving module {:?}: {:?}", path, err)
            }
            ServiceExecError::SerializeConfig { err } => {
                write!(f, "Error serializing config to toml: {:?}", err)
            }
            ServiceExecError::WriteConfig { path, err } => {
                write!(f, "Error saving config to {:?}: {:?}", path, err)
            }
            ServiceExecError::NoSuchBlueprint { path, err } => {
                write!(f, "Blueprint wasn't found at {:?}: {:?}", path, err)
            }
            ServiceExecError::IncorrectBlueprint { err } => {
                write!(f, "Error parsing blueprint: {:?}", err)
            }
            ServiceExecError::NoModuleConfig { path, err } => {
                write!(f, "Module config wasn't found at {:?}: {:?}", path, err)
            }
            ServiceExecError::IncorrectModuleConfig { err } => {
                write!(f, "Error parsing module config: {:?}", err)
            }
            ServiceExecError::WriteBlueprint { path, err } => {
                write!(f, "Error writing blueprint to {:?}: {:?}", path, err)
            }
        }
    }
}
