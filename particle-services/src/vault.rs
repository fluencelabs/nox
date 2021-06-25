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

use fs_utils::{create_dir, symlink_dir};
use std::path::Path;

pub fn create_vault(
    particle_id: &str,
    service_id: &str,
    workdir: &Path,
    vault_dir: &Path,
) -> Result<(), ServiceError> {
    let vault_path = vault_dir.join(particle_id);
    let symlink_path = workdir.join(service_id).join("tmp").join(particle_id);

    let result: std::io::Result<()> = try {
        create_dir(&vault_path)?;
        symlink_dir(&vault_path, &symlink_path)?;
    };

    result.map_err(|err| ServiceError::VaultCreation {
        err,
        particle_id: particle_id.to_string(),
        service_id: service_id.to_string(),
    })
}
