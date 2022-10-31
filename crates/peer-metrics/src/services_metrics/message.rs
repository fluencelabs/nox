use crate::ServiceType;
use fluence_app_service::MemoryStats;
use std::collections::HashMap;

pub type ModuleName = String;
pub type MemorySize = u64;

/// Service function call stats to store in memory
pub enum ServiceCallStats {
    Success {
        memory_delta_bytes: f64,
        call_time_sec: f64,
        timestamp: u64,
    },
    Fail {
        timestamp: u64,
    },
}

/// Messages to the metrics backend
pub enum ServiceMetricsMsg {
    Memory {
        service_id: String,
        service_type: ServiceType,
        memory_stat: ServiceMemoryStat,
    },
    CallStats {
        service_id: String,
        function_name: String,
        stats: ServiceCallStats,
    },
}

#[derive(Default)]
pub struct ServiceMemoryStat {
    /// Memory used by the service
    pub used_mem: MemorySize,
    /// Memory used by the modules that belongs to the service
    pub modules_stats: HashMap<ModuleName, MemorySize>,
}

impl ServiceMemoryStat {
    pub fn new(stats: &MemoryStats) -> ServiceMemoryStat {
        let mut modules_stats = HashMap::new();
        let mut used_mem: MemorySize = 0;
        for stat in &stats.0 {
            modules_stats.insert(stat.name.to_string(), stat.memory_size as MemorySize);
            used_mem += stat.memory_size as MemorySize;
        }
        ServiceMemoryStat {
            used_mem,
            modules_stats,
        }
    }
}
