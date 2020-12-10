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

use crate::error::ServiceError;
use crate::error::ServiceError::{
    CreateServicesDir, DeserializePersistedService, ReadPersistedService,
};

use config_utils::create_dirs;
use particle_modules::{is_service, list_files, service_file_name, ModuleError};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// TODO: all fields could be references, but I don't know how to achieve that
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistedService {
    pub service_id: String,
    pub blueprint_id: String,
    pub owner_id: Option<String>,
}

impl PersistedService {
    pub fn new(service_id: String, blueprint_id: String, owner_id: Option<String>) -> Self {
        Self {
            service_id,
            blueprint_id,
            owner_id,
        }
    }
}

/// Persist service info to disk, so it is recreated after restart
pub fn persist_service(
    services_dir: &PathBuf,
    service_id: String,
    blueprint_id: String,
    owner_id: Option<String>,
) -> Result<(), ModuleError> {
    use ModuleError::*;

    let path = services_dir.join(service_file_name(&service_id));
    let config = PersistedService::new(service_id, blueprint_id, owner_id);
    let bytes = toml::to_vec(&config).map_err(|err| SerializeConfig { err })?;
    std::fs::write(&path, bytes).map_err(|err| WriteConfig { path, err })
}

/// Load info about persisted services from disk, and create `AppService` for each of them
pub fn load_persisted_services(
    services_dir: &PathBuf,
) -> Vec<Result<PersistedService, ServiceError>> {
    // Load all persisted service file names
    let files = match list_files(services_dir) {
        Some(files) => files,
        None => {
            // Attempt to create directory and exit
            return create_dirs(&[&services_dir])
                .map_err(|err| CreateServicesDir {
                    path: services_dir.clone(),
                    err,
                })
                .err()
                .into_iter()
                .map(|err| Err(err))
                .collect();
        }
    };

    files
        .filter(is_service)
        .map(|file| {
            // Load service's persisted info
            let bytes = std::fs::read(&file).map_err(|err| ReadPersistedService {
                err,
                path: file.clone(),
            })?;
            let service =
                toml::from_slice(bytes.as_slice()).map_err(|err| DeserializePersistedService {
                    err,
                    path: file.clone(),
                })?;

            Ok(service)
        })
        .collect()
}
