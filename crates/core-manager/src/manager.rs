use crate::core_range::CoreRange;
use crate::core_set;
use crate::core_set::CoreSet;
use fxhash::{FxBuildHasher, FxHasher};
use parking_lot::Mutex;
use range_set_blaze::RangeSetBlaze;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash};
use std::ops::Deref;
use thiserror::Error;

type Map<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub struct CoreId(usize);
impl Deref for CoreId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct CoreManager {
    available_cores: CoreSet,
    core_type_state: Map<CoreType, CoreSet>,
    core_id_state: Map<CoreId, CoreType>,
    mutex: Mutex<()>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct UnitId(String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum CoreType {
    WorkerType(WorkerUnitType),
    Util,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum WorkerUnitType {
    CC(UnitId),
    Worker(UnitId),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to fetch core ids")]
    FailedGetCoreIds,
    #[error("Wrong cpu range {err}")]
    WrongCpuRange {
        #[source]
        err: core_set::Error,
    },
}

#[derive(Debug, Error)]
pub enum AssignError {
    #[error("Not found free cores")]
    NotFoundAvailableCores,
}

#[derive(Debug, Error)]
pub enum FreeError {
    #[error("Core {core_id:?} is already free")]
    NotFound { core_id: CoreId },
}

impl CoreManager {
    pub fn new(cpus_range: Option<CoreRange>) -> Result<Self, Error> {
        let available_cores: CoreSet = cpus_range
            .map(|range| range.try_into().map_err(|err| Error::WrongCpuRange { err }))
            .unwrap_or(Ok(CoreSet::default()))?;

        let core_type_state = HashMap::with_capacity_and_hasher(
            available_cores.0.len() as usize,
            FxBuildHasher::default(),
        );
        let core_id_state = HashMap::with_capacity_and_hasher(
            available_cores.0.len() as usize,
            FxBuildHasher::default(),
        );
        Ok(Self {
            available_cores,
            core_type_state,
            core_id_state,
            mutex: Mutex::new(()),
        })
    }

    pub fn assign_util_core(&mut self) -> Result<CoreId, AssignError> {
        let _guard = self.mutex.lock();
        assign(
            &mut self.available_cores,
            &mut self.core_id_state,
            &mut self.core_type_state,
            CoreType::Util,
        )
    }
    pub fn assign_worker_core(
        &mut self,
        worker_type: WorkerUnitType,
    ) -> Result<CoreId, AssignError> {
        match worker_type {
            WorkerUnitType::CC(unit_id) => self.switch_or_assign(
                WorkerUnitType::Worker(unit_id.clone()),
                WorkerUnitType::CC(unit_id),
            ),
            WorkerUnitType::Worker(unit_id) => self.switch_or_assign(
                WorkerUnitType::CC(unit_id.clone()),
                WorkerUnitType::Worker(unit_id),
            ),
        }
    }

    pub fn free(&mut self, core_id: CoreId) -> Result<(), FreeError> {
        let _guard = self.mutex.lock();
        let core_type = self.core_id_state.remove(&core_id);
        match core_type {
            None => Err(FreeError::NotFound { core_id }),
            Some(core_type) => {
                let set = self
                    .core_type_state
                    .get_mut(&core_type)
                    .expect("Non empty state");
                if set.0.len() == 1 {
                    self.core_type_state.remove(&core_type);
                } else {
                    set.0.remove(core_id.0);
                }
                self.available_cores.0.insert(core_id.0);
                Ok(())
            }
        }
    }

    fn switch_or_assign(
        &mut self,
        src: WorkerUnitType,
        dsc: WorkerUnitType,
    ) -> Result<CoreId, AssignError> {
        let _guard = self.mutex.lock();
        let state = self.core_type_state.get_mut(&CoreType::WorkerType(src));
        match state {
            None => assign(
                &mut self.available_cores,
                &mut self.core_id_state,
                &mut self.core_type_state,
                CoreType::WorkerType(dsc),
            ),
            Some(state) => {
                let core_id = state.0.pop_first().expect("Non empty state");
                let core_id = CoreId(core_id);
                save_state(
                    &mut self.core_id_state,
                    &mut self.core_type_state,
                    core_id.clone(),
                    CoreType::WorkerType(dsc),
                );
                Ok(core_id)
            }
        }
    }
}

fn assign(
    available_cores: &mut CoreSet,
    core_id_state: &mut Map<CoreId, CoreType>,
    core_type_state: &mut Map<CoreType, CoreSet>,
    core_type: CoreType,
) -> Result<CoreId, AssignError> {
    let core_id = available_cores.0.pop_first();
    match core_id {
        None => Err(AssignError::NotFoundAvailableCores),
        Some(core_id) => {
            let core_id = CoreId(core_id);
            save_state(core_id_state, core_type_state, core_id.clone(), core_type);
            Ok(core_id)
        }
    }
}

fn save_state(
    core_id_state: &mut Map<CoreId, CoreType>,
    core_type_state: &mut Map<CoreType, CoreSet>,
    core_id: CoreId,
    core_type: CoreType,
) {
    let mut current_ids = core_type_state
        .get(&core_type)
        .cloned()
        .unwrap_or(CoreSet(RangeSetBlaze::default()));
    current_ids.insert(core_id.0);
    core_id_state.insert(core_id, core_type.clone());
    core_type_state.insert(core_type, current_ids);
}

#[cfg(test)]
mod tests {
    use crate::manager::{UnitId, WorkerUnitType};
    use crate::CoreManager;

    #[test]
    fn test() {
        let mut manager = CoreManager::new(None).unwrap();
        let core = manager.assign_util_core().unwrap();
        println!("{:?}", core);
        let core = manager.assign_util_core().unwrap();
        println!("{:?}", core);
        let core = manager.assign_util_core().unwrap();
        println!("{:?}", core);
        manager.free(core).unwrap();
        println!("{:?}", manager.available_cores);
        println!("{:?}", manager.core_id_state);
        println!("{:?}", manager.core_type_state);
        let _core = manager
            .assign_worker_core(WorkerUnitType::Worker(UnitId("1".to_string())))
            .unwrap();
        println!("{:?}", manager.available_cores);
        println!("{:?}", manager.core_id_state);
        println!("{:?}", manager.core_type_state);
        let _core = manager
            .assign_worker_core(WorkerUnitType::CC(UnitId("1".to_string())))
            .unwrap();
        println!("{:?}", manager.available_cores);
        println!("{:?}", manager.core_id_state);
        println!("{:?}", manager.core_type_state);
    }
}
