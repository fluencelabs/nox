/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use crate::distributor::CoreDistributorState;
use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
use futures::StreamExt;
use hex_utils::serde_as::Hex;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

use crate::errors::PersistError;
use crate::types::WorkType;
use crate::Map;

pub(crate) trait StatePersister: Send + Sync {
    fn persist(&self) -> Result<(), PersistError>;
}

pub struct PersistenceTask {
    persister: Arc<dyn StatePersister>,
    receiver: Receiver<()>,
}

impl PersistenceTask {
    pub(crate) fn new(persistence: Arc<dyn StatePersister>, receiver: Receiver<()>) -> Self {
        Self {
            persister: persistence,
            receiver,
        }
    }
}

impl PersistenceTask {
    async fn process_events(self) {
        let stream = ReceiverStream::from(self.receiver);
        // We are not interested in the content of the event
        // We are waiting for the event to initiate the persistence process
        stream.for_each(move |_| {
            let persister = self.persister.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                        let result =  persister.persist();
                        match result {
                            Ok(_) => {
                                tracing::debug!(target: "core-distributor", "Core state was persisted");
                            }
                            Err(err) => {
                                tracing::warn!(target: "core-distributor", "Failed to save core state {err}");
                            }
                        }
                })
                    .await
                    .expect("Could not spawn persist task")
            }
        }).await;
    }

    pub async fn run(self) {
        tokio::task::Builder::new()
            .name("core-distributor-persist")
            .spawn(self.process_events())
            .expect("Could not spawn persist task");
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct PersistentCoreDistributorState {
    pub cores_mapping: Vec<(PhysicalCoreId, LogicalCoreId)>,
    pub system_cores: Vec<PhysicalCoreId>,
    pub available_cores: Vec<PhysicalCoreId>,
    #[serde_as(as = "Vec<(_, Hex)>")]
    pub unit_id_mapping: Vec<(PhysicalCoreId, CUID)>,
    #[serde_as(as = "Vec<(Hex, _)>")]
    pub work_type_mapping: Vec<(CUID, WorkType)>,
    #[serde_as(as = "Vec<(Hex, _)>")]
    pub cpu_cache: Map<CUID, PhysicalCoreId>,
}

impl PersistentCoreDistributorState {
    pub fn persist(&self, file_path: &Path) -> Result<(), PersistError> {
        let toml = toml::to_string_pretty(&self)
            .map_err(|err| PersistError::SerializationError { err })?;
        let mut file = File::create(file_path).map_err(|err| PersistError::IoError { err })?;
        file.write(toml.as_bytes())
            .map_err(|err| PersistError::IoError { err })?;
        Ok(())
    }
}

impl From<&CoreDistributorState> for PersistentCoreDistributorState {
    fn from(value: &CoreDistributorState) -> Self {
        Self {
            cores_mapping: value.cores_mapping.iter().map(|(k, v)| (*k, *v)).collect(),
            system_cores: value.system_cores.clone(),
            available_cores: value.available_cores.iter().cloned().collect(),
            unit_id_mapping: value
                .unit_id_mapping
                .iter()
                .map(|(k, v)| (*k, (*v)))
                .collect(),
            work_type_mapping: value
                .work_type_mapping
                .iter()
                .map(|(k, v)| ((*k), *v))
                .collect(),
            cpu_cache: value.cuid_cache.iter().map(|(k, v)| ((*k), *v)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::persistence::PersistentCoreDistributorState;
    use crate::types::WorkType;
    use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
    use hex::FromHex;

    #[test]
    fn test_serde() {
        let init_id_1 =
            <CUID>::from_hex("54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea")
                .unwrap();
        let persistent_state = PersistentCoreDistributorState {
            cores_mapping: vec![
                (PhysicalCoreId::new(1), LogicalCoreId::new(1)),
                (PhysicalCoreId::new(1), LogicalCoreId::new(2)),
                (PhysicalCoreId::new(2), LogicalCoreId::new(3)),
                (PhysicalCoreId::new(2), LogicalCoreId::new(4)),
                (PhysicalCoreId::new(3), LogicalCoreId::new(5)),
                (PhysicalCoreId::new(3), LogicalCoreId::new(6)),
                (PhysicalCoreId::new(4), LogicalCoreId::new(7)),
                (PhysicalCoreId::new(4), LogicalCoreId::new(8)),
            ],
            system_cores: vec![PhysicalCoreId::new(1)],
            available_cores: vec![PhysicalCoreId::new(2), PhysicalCoreId::new(3)],
            unit_id_mapping: vec![(PhysicalCoreId::new(4), init_id_1)],
            work_type_mapping: vec![(init_id_1, WorkType::Deal)],
            cpu_cache: Default::default(),
        };
        let actual = toml::to_string(&persistent_state).unwrap();
        let expected = "cores_mapping = [[1, 1], [1, 2], [2, 3], [2, 4], [3, 5], [3, 6], [4, 7], [4, 8]]\n\
        system_cores = [1]\n\
        available_cores = [2, 3]\n\
        unit_id_mapping = [[4, \"54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea\"]]\n\
        work_type_mapping = [[\"54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea\", \"Deal\"]]\n\
        cpu_cache = []\n";
        assert_eq!(expected, actual)
    }
}
