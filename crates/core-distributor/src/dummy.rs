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

use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
use fxhash::FxBuildHasher;

use crate::errors::AcquireError;
use crate::types::{AcquireRequest, Assignment, Cores, SystemAssignment};
use crate::{CoreDistributor, Map};

pub struct DummyCoreDistibutor;

impl Default for DummyCoreDistibutor {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyCoreDistibutor {
    pub fn new() -> Self {
        Self {}
    }
}

impl CoreDistributor for DummyCoreDistibutor {
    fn acquire_worker_cores(
        &self,
        acquire_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        let mut data = Map::with_hasher(FxBuildHasher::default());
        for unit_id in acquire_request.unit_ids {
            data.insert(
                unit_id,
                Cores {
                    physical_core_id: PhysicalCoreId::new(0),
                    logical_core_ids: vec![LogicalCoreId::new(0)],
                },
            );
        }

        Ok(Assignment::new(data))
    }

    fn release_worker_cores(&self, _unit_ids: &[CUID]) {}

    fn get_system_cpu_assignment(&self) -> SystemAssignment {
        SystemAssignment::new(vec![PhysicalCoreId::new(0)], vec![LogicalCoreId::new(0)])
    }
}
