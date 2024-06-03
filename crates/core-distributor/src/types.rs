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

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter, Write};

use ccp_shared::types::CUID;
use cpu_utils::pinning::ThreadPinner;
use cpu_utils::{LogicalCoreId, PhysicalCoreId};
use serde::{Deserialize, Serialize};

use crate::Map;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub enum WorkType {
    CapacityCommitment,
    Deal,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AcquireRequest {
    pub(crate) unit_ids: Vec<CUID>,
    pub(crate) worker_type: WorkType,
}

impl AcquireRequest {
    pub fn new(unit_ids: Vec<CUID>, worker_type: WorkType) -> Self {
        Self {
            unit_ids,
            worker_type,
        }
    }
}

impl Display for AcquireRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("{ unit_ids: ")?;
        f.write_char('[')?;
        for cuid in &self.unit_ids {
            f.write_str(format!("{}", cuid).as_str())?;
        }
        f.write_str("], ")?;
        f.write_str("worker_type: ")?;
        f.write_str(format!("{:?}", self.worker_type).as_str())?;
        f.write_str(" }")?;
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Cores {
    pub physical_core_id: PhysicalCoreId,
    pub logical_core_ids: Vec<LogicalCoreId>,
}

//TODO: enum
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Assignment {
    // We don't need a cryptographically secure hash and it is better to use a fx hash here
    // to improve performance
    pub cuid_cores: Map<CUID, Cores>,
}

pub struct SystemAssignment {
    pub physical_core_ids: Vec<PhysicalCoreId>,
    pub logical_core_ids: Vec<LogicalCoreId>,
}

impl SystemAssignment {
    pub(crate) fn new(
        physical_core_ids: Vec<PhysicalCoreId>,
        logical_core_ids: Vec<LogicalCoreId>,
    ) -> Self {
        Self {
            physical_core_ids,
            logical_core_ids,
        }
    }
    pub fn pin_current_thread_with(&self, thread_pinner: &dyn ThreadPinner) {
        thread_pinner.pin_current_thread_to_cpuset(&self.logical_core_ids);
    }
}

impl Assignment {
    pub(crate) fn new(cuid_cores: Map<CUID, Cores>) -> Self {
        Self { cuid_cores }
    }

    pub fn logical_core_ids(&self) -> Vec<LogicalCoreId> {
        self.cuid_cores
            .iter()
            .flat_map(|(_, cores)| cores.logical_core_ids.clone())
            .collect::<BTreeSet<LogicalCoreId>>() // needed to remove duplicates
            .into_iter()
            .collect::<Vec<_>>()
    }

    pub fn pin_current_thread_with(&self, thread_pinner: &dyn ThreadPinner) {
        thread_pinner.pin_current_thread_to_cpuset(&self.logical_core_ids());
    }
}
