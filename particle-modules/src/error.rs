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

use base64::DecodeError;
use fluence_app_service::{MarineError, TomlMarineNamedModuleConfig};
use marine_it_parser::ITParserError;
use serde_json::Value as JValue;
use thiserror::Error;

use json_utils::err_as_value;
use service_modules::Blueprint;

pub(super) type Result<T> = std::result::Result<T, ModuleError>;

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("Error saving module {path:?}: {err}")]
    AddModule {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error serializing config to toml: {err} {config:?}")]
    SerializeConfig {
        #[source]
        err: toml::ser::Error,
        config: TomlMarineNamedModuleConfig,
    },
    #[error("Error serializing blueprint to toml: {err} {blueprint:?}")]
    SerializeBlueprint {
        #[source]
        err: toml::ser::Error,
        blueprint: Blueprint,
    },
    #[error("Error serializing persisted service config to toml: {err} {config:?}")]
    SerializePersistedService {
        #[source]
        err: toml::ser::Error,
        config: Box<dyn Debug + Send + Sync>,
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
    #[error("Blueprint '{id}' has empty list of dependencies")]
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
    #[error("Error converting TomlMarineNamedModuleConfig to FaaSModuleConfig: {err}")]
    ModuleConvertError {
        #[source]
        err: MarineError,
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
        err: ITParserError,
    },
    #[error("Module with name {0} wasn't found, consider using module hash instead of a name")]
    InvalidModuleName(String),
    #[error("Expected module reference of format hash:xx got {reference}. Context: calculating blueprint hash")]
    InvalidModuleReference { reference: String },
    #[error("Error while decoding module bytes from base64: {err}")]
    ModuleInvalidBase64 {
        #[source]
        #[from]
        err: DecodeError,
    },
    #[error("Invalid module path {module_path:?}: {err}")]
    InvalidModulePath {
        module_path: String,
        #[source]
        err: eyre::Report,
    },
    #[error("Invalid module config path {config_path:?}: {err}")]
    InvalidModuleConfigPath {
        config_path: String,
        #[source]
        err: eyre::Report,
    },
    #[error("Vault directory for this particle doesn't exist. You must call a service first.")]
    VaultDoesNotExist { vault_path: PathBuf },
    #[error("Module not found in vault at {module_path:?}")]
    ModuleNotFoundInVault {
        module_path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Config not found in vault at {config_path:?}")]
    ConfigNotFoundInVault {
        config_path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error parsing module config from vault {config_path:?}: {err}")]
    IncorrectVaultModuleConfig {
        config_path: PathBuf,
        #[source]
        err: serde_json::Error,
    },
    #[error("Invalid blueprint path {blueprint_path:?}: {err}")]
    InvalidBlueprintPath {
        blueprint_path: String,
        #[source]
        err: eyre::Report,
    },
    #[error("Blueprint not found in vault at {blueprint_path:?}")]
    BlueprintNotFoundInVault {
        blueprint_path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error parsing blueprint from vault {blueprint_path:?}: {err}")]
    IncorrectVaultBlueprint {
        blueprint_path: PathBuf,
        #[source]
        err: serde_json::Error,
    },

    #[error(
    "Config error: max_heap_size = '{max_heap_size_wanted}' can't be bigger than {max_heap_size_allowed}'"
    )]
    MaxHeapSizeOverflow {
        max_heap_size_wanted: u64,
        max_heap_size_allowed: u64,
    },
}

impl From<ModuleError> for JValue {
    fn from(err: ModuleError) -> Self {
        err_as_value(err)
    }
}
