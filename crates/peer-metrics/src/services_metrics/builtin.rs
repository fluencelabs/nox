use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use parking_lot::RwLock;
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
/// The number of stored observations is now a constant MAX_METRICS_STORAGE_SIZE.
#[derive(Default, Debug, Clone, Serialize)]
pub struct NumericSeriesStat {
    /// Last N observations
    pub series: VecDeque<f64>,
    /// Sum of all observations
    pub total: f64,
    /// Average number of observations
    pub avg: f64,
}

impl NumericSeriesStat {
    /// Update the stat with new `value`.
    /// `count` is a total number of obserations that is stored outside.
    fn update(&mut self, max_metrics_storage_size: usize, value: f64, count: u64) {
        let count = count as f64;
        if self.series.len() >= max_metrics_storage_size {
            self.series.pop_front();
        }
        self.series.push_back(value);
        self.total += value;
        self.avg = (self.avg * count + value) / (count + 1.0);
    }
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct TimestampSeries {
    #[serde(rename = "timestamps")]
    pub series: VecDeque<u64>,
}

impl TimestampSeries {
    fn update(&mut self, max_metrics_storage_size: usize, value: u64) {
        if self.series.len() >= max_metrics_storage_size {
            self.series.pop_front();
        }
        self.series.push_back(value);
    }
}

/// All stats of the observed entity (service/function).
#[derive(Default, Debug, Clone, Serialize)]
pub struct Stats {
    /// Count of sucessful requests to the entity
    pub success_req_count: u64,
    /// Count of failed requests
    pub failed_req_count: u64,
    /// Memory increasing rate
    pub memory_deltas_bytes: NumericSeriesStat,
    /// Call execution time
    pub call_time_sec: NumericSeriesStat,
    #[serde(flatten)]
    /// Timestamps of last several calls
    pub timestamps: TimestampSeries,
}

impl Stats {
    fn update(&mut self, max_metrics_storage_size: usize, stats: &ServiceCallStats) {
        match stats {
            ServiceCallStats::Success {
                memory_delta_bytes,
                call_time_sec,
                timestamp,
            } => {
                self.memory_deltas_bytes.update(
                    max_metrics_storage_size,
                    *memory_delta_bytes,
                    self.success_req_count,
                );
                self.call_time_sec.update(
                    max_metrics_storage_size,
                    *call_time_sec,
                    self.success_req_count,
                );
                self.success_req_count += 1;
                self.timestamps.update(max_metrics_storage_size, *timestamp);
            }
            ServiceCallStats::Fail { timestamp } => {
                self.timestamps.update(max_metrics_storage_size, *timestamp);
                self.failed_req_count += 1;
            }
        }
    }
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct ServiceStat {
    /// Stats for the whole service
    pub total_stats: Stats,
    /// Stats for each interface function of the service.
    #[serde(serialize_with = "function_stats_ser")]
    pub functions_stats: HashMap<Name, Stats>,
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
    max_metrics_storage_size: usize,
}

impl ServicesMetricsBuiltin {
    pub fn new(max_metrics_storage_size: usize) -> Self {
        ServicesMetricsBuiltin {
            content: Arc::new(RwLock::new(HashMap::new())),
            max_metrics_storage_size,
        }
    }

    pub fn update(&self, service_id: ServiceId, function_name: Name, stats: ServiceCallStats) {
        let mut content = self.content.write();
        let service_stat = content.entry(service_id).or_default();
        let function_stat = service_stat
            .functions_stats
            .entry(function_name)
            .or_default();

        function_stat.update(self.max_metrics_storage_size, &stats);
        service_stat
            .total_stats
            .update(self.max_metrics_storage_size, &stats);
    }

    pub fn read(&self, service_id: &ServiceId) -> Option<ServiceStat> {
        let content = self.content.read();
        content.get(service_id).cloned()
    }

    pub fn get_used_memory(stats: &MemoryStats) -> u64 {
        stats.0.iter().fold(0, |acc, x| acc + x.memory_size as u64)
    }
}
