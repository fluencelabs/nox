use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use types::unit_id::UnitId;

#[serde_as]
#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhysicalCoreId(#[serde_as(as = "DisplayFromStr")] pub(crate) usize);

impl PhysicalCoreId {
    pub fn new(value: usize) -> Self {
        Self(value)
    }
}

#[serde_as]
#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord, Serialize, Deserialize)]
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
