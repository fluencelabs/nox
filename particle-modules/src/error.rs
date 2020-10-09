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
use std::path::PathBuf;

pub(super) type Result<T> = std::result::Result<T, ModuleError>;

#[derive(Debug)]
pub enum ModuleError {
    AddModule { path: PathBuf, err: std::io::Error },
    SerializeConfig { err: toml::ser::Error },
    WriteConfig { path: PathBuf, err: std::io::Error },
    NoSuchBlueprint { path: PathBuf, err: std::io::Error },
    IncorrectBlueprint { err: toml::de::Error },
    NoModuleConfig { path: PathBuf, err: std::io::Error },
    IncorrectModuleConfig { err: toml::de::Error },
    ModuleConvertError { err: FaaSError },
    WriteBlueprint { path: PathBuf, err: std::io::Error },
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleError::AddModule { path, err } => {
                write!(f, "Error saving module {:?}: {:?}", path, err)
            }
            ModuleError::SerializeConfig { err } => {
                write!(f, "Error serializing config to toml: {:?}", err)
            }
            ModuleError::WriteConfig { path, err } => {
                write!(f, "Error saving config to {:?}: {:?}", path, err)
            }
            ModuleError::NoSuchBlueprint { path, err } => {
                write!(f, "Blueprint wasn't found at {:?}: {:?}", path, err)
            }
            ModuleError::IncorrectBlueprint { err } => {
                write!(f, "Error parsing blueprint: {:?}", err)
            }
            ModuleError::NoModuleConfig { path, err } => {
                write!(f, "Module config wasn't found at {:?}: {:?}", path, err)
            }
            ModuleError::IncorrectModuleConfig { err } => {
                write!(f, "Error parsing module config: {:?}", err)
            }
            ModuleError::WriteBlueprint { path, err } => {
                write!(f, "Error writing blueprint to {:?}: {:?}", path, err)
            }
            ModuleError::ModuleConvertError { err } => write!(
                f,
                "Error converting TomlFaaSNamedModuleConfig to FaaSModuleConfig: {:?}",
                err
            ),
        }
    }
}
