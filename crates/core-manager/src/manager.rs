use crate::DevCoreManager;
use ccp_shared::types::CUID;
use enum_dispatch::enum_dispatch;

use crate::dummy::DummyCoreManager;
use crate::errors::AcquireError;
use crate::strict::StrictCoreManager;
use crate::types::{AcquireRequest, Assignment};

/// The `CoreManagerFunctions` trait defines operations for managing CPU cores.
///
/// Implement this trait to enable core acquisition, release, retrieval of system CPU assignments,
/// and persistence of the core manager's state.
///
/// # Trait Functions:
///
/// - `acquire_worker_core(assign_request: AcquireRequest) -> Result<Assignment, AcquireError>`:
///   Acquires CPU cores for a set of unit IDs and a specified worker type.
///
/// - `release(unit_ids: Vec<UnitId>)`:
///   Releases previously acquired CPU cores associated with a set of unit IDs.
///
/// - `get_system_cpu_assignment() -> Assignment`:
///   Retrieves the system's CPU assignment, including physical and logical core IDs.
///
/// - `persist() -> Result<(), PersistError>`:
///   Persists the current state of the core manager to an external storage location.
///
/// # Implementing Types:
///
/// - [`PersistentCoreManager`](struct.PersistentCoreManager.html):
///   Manages CPU cores persistently by saving and loading state to/from a file path.
///
/// - [`DummyCoreManager`](struct.DummyCoreManager.html):
///   Provides a dummy implementation for non-persistent core management scenarios.
///
/// - [`CoreManager`](enum.CoreManager.html):
///   Enumerates persistent and dummy core managers, allowing flexible core management choices.
///
/// # Example Usage:
///
/// ```rust
/// use core_manager::{CoreManager, AcquireRequest, WorkType};
///
/// let (core_manager, persistence_task) = PersistentCoreManager::from_path("core_state.toml".into(), 2, CoreRange::default()).expect("Failed to create manager");
/// let unit_ids = vec!["1".into(), "2".into()];
///
/// // Acquire and release cores
/// let assignment = core_manager.acquire_worker_core(AcquireRequest { unit_ids, worker_type: WorkType::CapacityCommitment }).unwrap();
///
/// // Retrieve system CPU assignment
/// let system_assignment = core_manager.get_system_cpu_assignment();
///
/// // Run persistence task in the background
/// tokio::spawn(persistence_task.run(core_manager.clone()));
///
/// сore_manager.release(unit_ids);
/// ```
#[enum_dispatch]
pub trait CoreManagerFunctions {
    fn acquire_worker_core(
        &self,
        assign_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError>;

    fn release(&self, unit_ids: Vec<CUID>);

    fn get_system_cpu_assignment(&self) -> Assignment;
}

#[enum_dispatch(CoreManagerFunctions)]
pub enum CoreManager {
    Persistent(StrictCoreManager),
    Dev(DevCoreManager),
    Dummy(DummyCoreManager),
}
