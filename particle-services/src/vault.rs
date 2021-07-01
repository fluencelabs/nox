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

use host_closure::AVMEffect;

use std::path::PathBuf;

pub fn create_vault(
    effect: AVMEffect<PathBuf>,
    service_id: &str,
    particle_id: &str,
) -> Result<(), ServiceError> {
    effect().map_err(|err| ServiceError::VaultCreation {
        err,
        service_id: service_id.to_string(),
        particle_id: particle_id.to_string(),
    })?;

    Ok(())
}
