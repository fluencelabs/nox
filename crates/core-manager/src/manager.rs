use core_affinity::CoreId;
use dashmap::DashMap;
use fxhash::{FxBuildHasher, FxHasher};
use std::collections::BTreeSet;
use std::hash::BuildHasherDefault;
use thiserror::Error;
use crate::cpu_range::CpuRange;

type Map<K, V> = DashMap<K, V, BuildHasherDefault<FxHasher>>;
pub struct CoreManager {
    util_cores_num: usize,
    worker_cores_num: usize,
    core_ids: Vec<CoreId>,
    state: Map<CoreId, CoreType>,
}

pub struct UnitId(String);

pub enum CoreType {
    CC(UnitId),
    Worker(UnitId),
    Util,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to fetch core ids")]
    FailedGetCoreIds,
    #[error("Wrong cpu range requested")]
    WrongCpuRange {
        current_total: usize,
        range: BTreeSet<CoreId>,
    },
}



impl CoreManager {
    pub fn new(
        util_cores_num: usize,
        cpus_range: Option<CpuRange>,
    ) -> Result<Self, Error> {
        let core_ids = core_affinity::get_core_ids().ok_or(Error::FailedGetCoreIds)?;

        if let Some(available_cores) = cpus_range {
           /* let last_core = core_ids.last().expect("core_ids cant be empty");
            let last_available_core = available_cores
                .iter()
                .next()
                .expect("Available cores can't be empty");
            if last_available_core.gt(last_core) {
                return Err(Error::WrongCpuRange {
                    current_total: core_ids.len(),
                    range: available_cores
                });
            }*/
        }


        let state = DashMap::with_hasher(FxBuildHasher::default());
        Ok(Self {
            util_cores_num: 0,
            worker_cores_num: 0,
            state,
            core_ids,
        })
    }
}
