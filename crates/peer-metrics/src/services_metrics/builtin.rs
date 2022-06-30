use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

use serde::{Serialize, ser::{Serializer, SerializeSeq}};
use serde_json;

use fluence_app_service::MemoryStats;

type ServiceId = String;
type Name = String;

#[derive(Default, Debug, Clone, Serialize)]
struct Stat {
    // Count of request to the entity
    req_count: u64,
    // Last N memory incearses
    deltas_bytes: VecDeque<u64>,
    // Total memory increases
    total_delta_bytes: u64,
    // Average memory increases per request to the entity
    delta_avg_bytes: u64,
}

impl Stat {
    const MAX_METRICS_STORAGE_SIZE: usize = 5;
    fn update(&mut self, delta: u64) {
        if self.deltas_bytes.len() >= Self::MAX_METRICS_STORAGE_SIZE {
            self.deltas_bytes.pop_front();
        }
        self.deltas_bytes.push_back(delta);
        self.total_delta_bytes += delta;
        self.delta_avg_bytes =
            (self.delta_avg_bytes * self.req_count + delta) / (self.req_count + 1);

        self.req_count += 1;
    }
}

#[derive(Default, Debug, Clone, Serialize)]
struct ServiceStat {
    total_stat: Stat,
    #[serde(serialize_with = "function_stats_ser")]
    functions_stats: HashMap<Name, Stat>,
}

fn function_stats_ser<S>(stats: &HashMap<Name, Stat>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(stats.len()))?;
    for (k, v) in stats {
        seq.serialize_element(&serde_json::json!({"name": k, "stat": v}))?;
    }
    seq.end()
}

#[derive(Clone)]
pub struct ServicesMetricsBuiltin {
    content: Arc<RwLock<HashMap<ServiceId, ServiceStat>>>,
}

impl ServicesMetricsBuiltin {
    pub fn new() -> Self {
        ServicesMetricsBuiltin {
            content: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn update(&self, service_id: ServiceId, function_name: Name, delta: u64) {
        let mut content = self.content.write().unwrap();
        let service_stat = content.entry(service_id).or_default();
        let function_stat = service_stat
            .functions_stats
            .entry(function_name)
            .or_default();

        function_stat.update(delta);
        service_stat.total_stat.update(delta);
    }

    pub fn read(&self, service_id: &ServiceId) -> Option<serde_json::Result<serde_json::Value>> {
        let content = self.content.write().unwrap();
        let stat = content.get(service_id)?;
        Some(serde_json::to_value(stat))
    }

    pub fn get_used_memory(stats: &MemoryStats) -> u64 {
        stats.0.iter().fold(0, |acc, x| acc + x.memory_size as u64)
    }

    pub fn debug_print(&self) {
        let content = self.content.read().unwrap();
        println!("SERVICES METRICS: {:?}", content);
    }
}
