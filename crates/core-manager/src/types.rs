use core_affinity::{set_mask_for_current, CoreId};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::BTreeSet;
use types::unit_id::UnitId;

#[serde_as]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhysicalCoreId(#[serde_as(as = "DisplayFromStr")] pub(crate) usize);

impl PhysicalCoreId {
    pub fn new(value: usize) -> Self {
        Self(value)
    }
}

#[serde_as]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LogicalCoreId(#[serde_as(as = "DisplayFromStr")] pub(crate) usize);

impl LogicalCoreId {
    pub fn new(value: usize) -> Self {
        Self(value)
    }
}

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
        let cores: Vec<CoreId> = self
            .logical_core_ids
            .iter()
            .map(|core_id| CoreId { id: core_id.0 })
            .collect();
        set_mask_for_current(&cores);
    }
}
