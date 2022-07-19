use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

use serde::{
    ser::{SerializeSeq, Serializer},
    Serialize,
};
use serde_json;

use fluence_app_service::MemoryStats;

use crate::services_metrics::message::ServiceCallStats;

type ServiceId = String;
type Name = String;

/// Store a part of series of numeric observations and some parameters that desribe the series.
/// The number of stored observations is now a constant Self::MAX_METRICS_STORAGE_SIZE.
#[derive(Default, Debug, Clone, Serialize)]
struct NumericSeriesStat {
    /// Last N observations
    series: VecDeque<f64>,
    /// Sum of all observations
    total: f64,
    /// Average number of observations
    avg: f64,
}

impl NumericSeriesStat {
    const MAX_METRICS_STORAGE_SIZE: usize = 5;

    /// Update the stat with new `value`.
    /// `count` is a total number of obserations that is stored outside.
    fn update(&mut self, value: f64, count: f64) {
        if self.series.len() >= Self::MAX_METRICS_STORAGE_SIZE {
            self.series.pop_front();
        }
        self.series.push_back(value);
        self.total += value;
        self.avg = (self.avg * count + value) / (count + 1.0);
    }
}

/// All stats of the observed entity (service/function).
#[derive(Default, Debug, Clone, Serialize)]
struct Stats {
    /// Count of sucessful requests to the entity
    success_req_count: u64,
    /// Count of failed requests
    failed_req_count: u64,
    /// Memory increasing rate
    memory_deltas_bytes: NumericSeriesStat,
    call_time_sec: NumericSeriesStat,
}

impl Stats {
    fn update(&mut self, stats: &ServiceCallStats) {
        match stats {
            ServiceCallStats::Success {
                memory_delta_bytes,
                call_time_sec,
            } => {
                self.memory_deltas_bytes
                    .update(*memory_delta_bytes, self.success_req_count as f64);
                self.call_time_sec
                    .update(*call_time_sec, self.success_req_count as f64);
                self.success_req_count += 1;
            }
            ServiceCallStats::Fail => {
                self.failed_req_count += 1;
            }
        }
    }
}

#[derive(Default, Debug, Clone, Serialize)]
struct ServiceStat {
    total_stats: Stats,
    #[serde(serialize_with = "function_stats_ser")]
    functions_stats: HashMap<Name, Stats>,
}

fn function_stats_ser<S>(stats: &HashMap<Name, Stats>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(stats.len()))?;
    for (k, v) in stats {
        seq.serialize_element(&serde_json::json!({"name": k, "stats": v}))?;
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

    pub fn update(&self, service_id: ServiceId, function_name: Name, stats: ServiceCallStats) {
        let mut content = self.content.write().unwrap();
        let service_stat = content.entry(service_id).or_default();
        let function_stat = service_stat
            .functions_stats
            .entry(function_name)
            .or_default();

        function_stat.update(&stats);
        service_stat.total_stats.update(&stats);
    }

    pub fn read(&self, service_id: &ServiceId) -> Option<serde_json::Result<serde_json::Value>> {
        let content = self.content.write().unwrap();
        let stat = content.get(service_id)?;
        Some(serde_json::to_value(stat))
    }

    pub fn get_used_memory(stats: &MemoryStats) -> u64 {
        stats.0.iter().fold(0, |acc, x| acc + x.memory_size as u64)
    }
}
