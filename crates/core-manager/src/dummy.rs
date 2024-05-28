/*
 * Copyright 2024 Fluence DAO
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

use crate::errors::AcquireError;
use crate::manager::CoreManagerFunctions;
use crate::types::{AcquireRequest, Assignment};
use crate::Map;
use async_trait::async_trait;
use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
use fxhash::FxBuildHasher;
use rand::prelude::IteratorRandom;
use std::collections::BTreeSet;

#[derive(Default)]
pub struct DummyCoreManager {}

impl DummyCoreManager {
    fn all_cores(&self) -> Assignment {
        let physical_core_ids = (0..num_cpus::get_physical())
            .map(|v| PhysicalCoreId::from(v as u32))
            .collect();
        let logical_core_ids = (0..num_cpus::get())
            .map(|v| LogicalCoreId::from(v as u32))
            .collect();
        Assignment {
            physical_core_ids,
            logical_core_ids,
            cuid_cores: Map::with_hasher(FxBuildHasher::default()),
        }
    }
}

#[async_trait]
impl CoreManagerFunctions for DummyCoreManager {
    fn acquire_worker_core(
        &self,
        assign_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        let all_cores = self.all_cores();

        let logical_core_ids: BTreeSet<LogicalCoreId> = BTreeSet::from_iter(
            all_cores
                .logical_core_ids
                .into_iter()
                .choose_multiple(&mut rand::thread_rng(), assign_request.unit_ids.len()),
        );

        Ok(Assignment {
            physical_core_ids: BTreeSet::new(),
            logical_core_ids,
            cuid_cores: Map::with_hasher(FxBuildHasher::default()),
        })
    }

    fn release(&self, _unit_ids: &[CUID]) {}

    fn get_system_cpu_assignment(&self) -> Assignment {
        self.all_cores()
    }
}
