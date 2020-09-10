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

use super::error::ServiceExecError::{self, *};
use super::ServiceCall;

use crate::app_service::error::ServiceExecError::{CreateServicesDir, ReadPersistedService};
use crate::app_service::files;
use crate::app_service::persisted_service::PersistedService;
use std::path::PathBuf;

use super::service::*;

impl AppServiceBehaviour {
    /// Load info about persisted services from disk, and create `AppService` for each of them
    pub fn create_persisted_services(&mut self) -> Vec<ServiceExecError> {
        // Load all persisted service file names
        let files = match Self::list_files(&self.config.services_dir) {
            Some(files) => files,
            None => {
                // Attempt to create directory and exit
                return std::fs::create_dir_all(&self.config.services_dir)
                    .map_err(|err| CreateServicesDir {
                        path: self.config.services_dir.clone(),
                        err,
                    })
                    .err()
                    .into_iter()
                    .collect();
            }
        };

        files.filter(files::is_service).map(|file| {
            // Load service's persisted info
            let bytes =
                std::fs::read(&file).map_err(|err| ReadPersistedService { err, path: file.clone() })?;
            let PersistedService { service_id, blueprint_id } =
                toml::from_slice(bytes.as_slice()).map_err(|err| IncorrectModuleConfig { err })?;

            // Don't overwrite existing services
            if self.app_services.contains_key(service_id) {
                log::warn!(
                    "Won't load persisted service {}: there's already a service with such service id",
                    service_id
                );

                return Ok(());
            }

            // Schedule creation of the service
            self.execute(ServiceCall::Create {
                service_id: Some(service_id.to_string()),
                blueprint_id: blueprint_id.to_string(),
                call: None
            });

            self.wake();

            Ok(())
        }).filter_map(|r| r.err()).collect()
    }

    /// Persist service info to disk, so it is recreated after restart
    pub(super) fn persist_service(
        services_dir: &PathBuf,
        service_id: &str,
        blueprint_id: &str,
    ) -> Result<()> {
        let config = PersistedService::new(service_id, blueprint_id);
        let bytes = toml::to_vec(&config).map_err(|err| SerializeConfig { err })?;
        let path = services_dir.join(files::service_file_name(service_id));
        std::fs::write(&path, bytes).map_err(|err| WriteConfig { path, err })
    }

    /// List files in directory
    pub(super) fn list_files(dir: &PathBuf) -> Option<impl Iterator<Item = PathBuf>> {
        let dir = std::fs::read_dir(dir).ok()?;
        Some(dir.filter_map(|p| p.ok()?.path().into()))
    }
}
