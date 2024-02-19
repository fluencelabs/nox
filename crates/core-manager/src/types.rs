use cpu_utils::pinning::pin_current_thread_to_cpuset;
use cpu_utils::{LogicalCoreId, PhysicalCoreId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use types::unit_id::UnitId;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub enum WorkType {
    CapacityCommitment,
    Deal,
}

pub struct AcquireRequest {
    pub(crate) unit_ids: Vec<UnitId>,
    pub(crate) worker_type: WorkType,
}

impl AcquireRequest {
    pub fn new(unit_ids: Vec<UnitId>, worker_type: WorkType) -> Self {
        Self {
            unit_ids,
            worker_type,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Assignment {
    pub physical_core_ids: BTreeSet<PhysicalCoreId>,
    pub logical_core_ids: BTreeSet<LogicalCoreId>,
}

impl Assignment {
    pub fn pin_current_thread(&self) {
        pin_current_thread_to_cpuset(self.logical_core_ids.iter().cloned());
    }
}
