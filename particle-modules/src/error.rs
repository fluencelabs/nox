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

use fluence_app_service::FaaSError;
use json_utils::err_as_value;

use fce_wit_parser::WITParserError;
use serde_json::Value as JValue;
use std::path::PathBuf;
use thiserror::Error;

pub(super) type Result<T> = std::result::Result<T, ModuleError>;

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("Error saving module {path:?}: {err}")]
    AddModule {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error serializing config to toml: {err}")]
    SerializeConfig {
        #[source]
        err: toml::ser::Error,
    },
    #[error("Error saving config to {path:?}: {err}")]
    WriteConfig {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Blueprint wasn't found at {path:?}: {err}")]
    NoSuchBlueprint {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Blueprint '{id}' wasn't found")]
    BlueprintNotFound { id: String },
    #[error("Blueprint '{id}' hash empty list of dependencies somehow")]
    EmptyDependenciesList { id: String },
    #[error("Blueprint '{id}' facade dependency is not a hash of a module")]
    FacadeShouldBeHash { id: String },
    #[error("Error parsing blueprint: {err}")]
    IncorrectBlueprint {
        #[source]
        err: toml::de::Error,
    },
    #[error("Module config wasn't found at {path:?}: {err}")]
    NoModuleConfig {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error parsing module config: {err}")]
    IncorrectModuleConfig {
        #[source]
        err: toml::de::Error,
    },
    #[error("Error writing blueprint to {path:?}: {err}")]
    WriteBlueprint {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error converting TomlFaaSNamedModuleConfig to FaaSModuleConfig: {err}")]
    ModuleConvertError {
        #[source]
        err: FaaSError,
    },
    #[error("Module wasn't found on path {path:?}: {err}")]
    ModuleNotFound {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Cannot read modules interface {path:?}: {err}")]
    ReadModuleInterfaceError {
        path: PathBuf,
        #[source]
        err: WITParserError,
    },
    #[error("Module with name {0} wasn't found, consider using module hash instead of a name")]
    InvalidModuleName(String),
    #[error("Expected module reference of format hash:xx got {reference}. Context: calculating blueprint hash")]
    InvalidModuleReference { reference: String },
}

impl From<ModuleError> for JValue {
    fn from(err: ModuleError) -> Self {
        err_as_value(err)
    }
}
