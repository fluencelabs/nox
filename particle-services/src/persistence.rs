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

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app_services::Service;
use crate::error::ServiceError;
use crate::ServiceError::{SerializePersistedService, WritePersistedService};
use crate::ServiceType;
use fluence_libp2p::{PeerId};
use service_modules::{is_service, service_file_name};
use types::peer_scope::PeerScope;
use types::peer_id;

// TODO: all fields could be references, but I don't know how to achieve that
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PersistedService {
    pub service_id: String,
    pub service_type: Option<ServiceType>,
    pub blueprint_id: String,
    #[serde(default)]
    // Old versions of PersistedService may omit `aliases` field, tolerate that
    pub aliases: Vec<String>,
    // Old versions of PersistedService may omit `owner` field, tolerate that via RandomPeerId::random
    #[serde(serialize_with = "peer_id::serde::serialize", deserialize_with = "peer_id::serde::deserialize")]
    pub owner_id: PeerId,
    pub peer_scope: PeerScope,
}

impl PersistedService {
    pub fn from_service(service: &Service) -> Self {
        PersistedService {
            service_id: service.service_id.clone(),
            service_type: Some(service.service_type.clone()),
            blueprint_id: service.blueprint_id.clone(),
            aliases: service.aliases.read().clone(),
            owner_id: service.owner_id,
            peer_scope: service.peer_scope,
        }
    }

    /// Persist service info to disk, so it is recreated after restart
    pub async fn persist(&self, services_dir: &Path) -> Result<(), ServiceError> {
        let path = services_dir.join(service_file_name(&self.service_id));
        let bytes = toml::to_vec(self).map_err(|err| SerializePersistedService {
            err,
            config: Box::new(self.clone()),
        })?;
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|err| WritePersistedService { path, err })
    }
}

/// Load info about persisted services from disk, and create `AppService` for each of them
pub async fn load_persisted_services(
    services_dir: &Path,
) -> eyre::Result<Vec<(PersistedService, PathBuf)>> {
    let services = fs_utils::load_persisted_data(services_dir, is_service, |bytes| {
        toml::from_slice(bytes).map_err(|e| e.into())
    })
    .await?;

    Ok(services)
}

pub async fn remove_persisted_service(
    services_dir: &Path,
    service_id: String,
) -> Result<(), std::io::Error> {
    tokio::fs::remove_file(services_dir.join(service_file_name(&service_id))).await
}

#[cfg(test)]
mod tests {
    use crate::persistence::{load_persisted_services, PersistedService};
    use fluence_libp2p::RandomPeerId;
    use types::peer_scope::PeerScope;

    #[tokio::test]
    async fn test_persistence() {
        let tmp_dir = tempfile::tempdir().expect("Could not get temp dir");
        let owner_id = RandomPeerId::random();
        let service_1 = PersistedService {
            service_id: "service_id_1".to_string(),
            service_type: None,
            blueprint_id: "blueprint_id_1".to_string(),
            aliases: vec!["alias_1".to_string()],
            owner_id,
            peer_scope: PeerScope::WorkerId(owner_id.into()),
        };
        service_1
            .persist(tmp_dir.path())
            .await
            .expect("Could not persist service");

        let service_2 = PersistedService {
            service_id: "service_id_2".to_string(),
            service_type: None,
            blueprint_id: "blueprint_id_2".to_string(),
            aliases: vec!["alias_2".to_string()],
            owner_id,
            peer_scope: PeerScope::Host,
        };
        service_2
            .persist(tmp_dir.path())
            .await
            .expect("Could not persist service");

        let result: Vec<PersistedService> = load_persisted_services(tmp_dir.path())
            .await
            .expect("Could not load persisted services")
            .into_iter()
            .map(|(s, _)| s)
            .collect();

        assert_eq!(result.len(), 2);
        assert!(result.contains(&service_1));
        assert!(result.contains(&service_2));
    }
}
