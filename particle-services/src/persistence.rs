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

use crate::app_services::Service;
use serde::{Deserialize, Serialize};
use std::path::Path;

// TODO: all fields could be references, but I don't know how to achieve that
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistedService {
    pub service_id: String,
    pub blueprint_id: String,
    #[serde(default)]
    // Old versions of PersistedService may omit `aliases` field, tolerate that
    pub aliases: Vec<String>,
    // Old versions of PersistedService may omit `owner` field, tolerate that
    #[serde(default)]
    pub owner_id: String,
}

impl PersistedService {
    pub fn new(
        service_id: String,
        blueprint_id: String,
        aliases: Vec<String>,
        owner_id: String,
    ) -> Self {
        Self {
            service_id,
            blueprint_id,
            aliases,
            owner_id,
        }
    }

    pub fn from_service(service_id: String, service: &Service) -> Self {
        PersistedService::new(
            service_id,
            service.blueprint_id.clone(),
            service.aliases.clone(),
            service.owner_id.clone(),
        )
    }
}

/// Persist service info to disk, so it is recreated after restart
pub fn persist_service(
    services_dir: &Path,
    persisted_service: PersistedService,
) -> Result<(), ModuleError> {
    use ModuleError::*;

    let path = services_dir.join(service_file_name(&persisted_service.service_id));
    let bytes = toml::to_vec(&persisted_service).map_err(|err| SerializeConfig { err })?;
    std::fs::write(&path, bytes).map_err(|err| WriteConfig { path, err })
}

/// Load info about persisted services from disk, and create `AppService` for each of them
pub fn load_persisted_services(services_dir: &Path) -> Vec<Result<PersistedService, ServiceError>> {
    // Load all persisted service file names
    let files = match list_files(services_dir) {
        Some(files) => files,
        None => {
            // Attempt to create directory and exit
            return create_dirs(&[&services_dir])
                .map_err(|err| CreateServicesDir {
                    path: services_dir.to_path_buf(),
                    err,
                })
                .err()
                .into_iter()
                .map(Err)
                .collect();
        }
    };

    files
        .filter(|p| is_service(&p))
        .map(|file| {
            // Load service's persisted info
            let bytes = std::fs::read(&file).map_err(|err| ReadPersistedService {
                err,
                path: file.to_path_buf(),
            })?;
            let service =
                toml::from_slice(bytes.as_slice()).map_err(|err| DeserializePersistedService {
                    err,
                    path: file.to_path_buf(),
                })?;

            Ok(service)
        })
        .collect()
}
