/*
 * Copyright 2021 Fluence Labs Limited
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

use crate::ServiceError;

use fs_utils::symlink_dir;
use host_closure::AVMEffect;

use std::path::{Path, PathBuf};

pub fn create_vault(
    effect: AVMEffect<PathBuf>,
    service_id: &str,
    particle_id: &str,
    services_workdir: &Path,
) -> Result<(), ServiceError> {
    let vault_dir = effect().map_err(|err| ServiceError::VaultCreation {
        err,
        service_id: service_id.to_string(),
        particle_id: particle_id.to_string(),
    })?;

    link_vault(particle_id, service_id, services_workdir, &vault_dir)
}

fn link_vault(
    particle_id: &str,
    service_id: &str,
    services_workdir: &Path,
    vault_dir: &Path,
) -> Result<(), ServiceError> {
    // Will be visible as /tmp/$particle_id inside the service
    let symlink_path = services_workdir
        .join(service_id)
        .join("tmp")
        .join(particle_id);

    symlink_dir(vault_dir, &symlink_path).map_err(|err| ServiceError::VaultLink {
        err,
        particle_id: particle_id.to_string(),
        service_id: service_id.to_string(),
    })
}
