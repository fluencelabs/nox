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

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter, Write};

use ccp_shared::types::CUID;
use cpu_utils::pinning::ThreadPinner;
use cpu_utils::{LogicalCoreId, PhysicalCoreId};
use serde::{Deserialize, Serialize};

use crate::Map;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum WorkType {
    CapacityCommitment,
    Deal,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AcquireRequest {
    pub(crate) unit_ids: Vec<CUID>,
    pub(crate) work_type: WorkType,
}

impl AcquireRequest {
    pub fn new(unit_ids: Vec<CUID>, worker_type: WorkType) -> Self {
        Self {
            unit_ids,
            work_type: worker_type,
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
        f.write_str(format!("{:?}", self.work_type).as_str())?;
        f.write_str(" }")?;
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Cores {
    pub physical_core_id: PhysicalCoreId,
    pub logical_core_ids: Vec<LogicalCoreId>,
}

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

    pub fn physical_core_count(&self) -> usize {
        self.cuid_cores
            .values()
            .map(|cores| cores.physical_core_id)
            .collect::<BTreeSet<PhysicalCoreId>>()
            .len()
    }

    pub fn pin_current_thread_with(&self, thread_pinner: &dyn ThreadPinner) {
        thread_pinner.pin_current_thread_to_cpuset(&self.logical_core_ids());
    }
}
