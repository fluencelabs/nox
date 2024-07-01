/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use prometheus_client::collector::Collector;
use prometheus_client::encoding::{DescriptorEncoder, EncodeMetric, MetricEncoder};
use prometheus_client::metrics::counter::ConstCounter;
use prometheus_client::metrics::gauge::ConstGauge;
use prometheus_client::metrics::MetricType;
use std::fmt::Error;
use std::ops::Range;
use std::time::Duration;
use tokio::runtime::RuntimeMetrics;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct TokioCollector {
    metrics: RuntimeMetrics,
}

impl TokioCollector {
    pub fn new() -> Self {
        let handle = tokio::runtime::Handle::current();
        let metrics = handle.metrics();
        Self { metrics }
    }
}
const WORKER_LABEL: &str = "worker";

#[derive(Debug)]
struct TokioWorkerHistogram {
    sum: f64,
    count: u64,
    data: Vec<(f64, u64)>,
}

impl EncodeMetric for TokioWorkerHistogram {
    fn encode(&self, mut encoder: MetricEncoder<'_>) -> std::result::Result<(), Error> {
        encoder.encode_histogram::<()>(self.sum, self.count, &self.data, None)
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Histogram
    }
}
struct TokioHistogramBuckets(Vec<Range<Duration>>);

impl Collector for TokioCollector {
    fn encode(&self, mut encoder: DescriptorEncoder<'_>) -> Result<(), Error> {
        let workers_count = self.metrics.num_workers();

        let histogram_buckets = if self.metrics.poll_count_histogram_enabled() {
            let num_buckets = self.metrics.poll_count_histogram_num_buckets();
            let bucket_ranges: Vec<Range<Duration>> = (0..num_buckets)
                .map(|index| self.metrics.poll_count_histogram_bucket_range(index))
                .collect();
            Some(TokioHistogramBuckets(bucket_ranges))
        } else {
            None
        };

        let workers = ConstGauge::new(workers_count as i64);

        let workers_encoder = encoder.encode_descriptor(
            "workers",
            "The number of worker threads used by the runtime",
            None,
            workers.metric_type(),
        )?;

        workers.encode(workers_encoder)?;

        let active_tasks = ConstGauge::new(self.metrics.active_tasks_count() as i64);

        let active_tasks_encoder = encoder.encode_descriptor(
            "active_tasks",
            "The number of active tasks in the runtime",
            None,
            active_tasks.metric_type(),
        )?;

        active_tasks.encode(active_tasks_encoder)?;

        let num_blocking_threads = ConstGauge::new(self.metrics.num_blocking_threads() as i64);

        let num_blocking_threads_encoder = encoder.encode_descriptor(
            "num_blocking_threads",
            "Еhe number of additional blocking threads spawned by the runtime",
            None,
            num_blocking_threads.metric_type(),
        )?;

        num_blocking_threads.encode(num_blocking_threads_encoder)?;

        let num_idle_blocking_threads =
            ConstGauge::new(self.metrics.num_idle_blocking_threads() as i64);

        let num_idle_blocking_threads_encoder = encoder.encode_descriptor(
            "num_idle_blocking_threads",
            "Returns the number of idle blocking threads, which have spawned by the runtime",
            None,
            num_idle_blocking_threads.metric_type(),
        )?;

        num_idle_blocking_threads.encode(num_idle_blocking_threads_encoder)?;

        let remote_schedule_count = ConstCounter::new(self.metrics.remote_schedule_count());

        let remote_schedule_count_encoder = encoder.encode_descriptor(
            "remote_schedule",
            "Returns the number of tasks scheduled from outside of the runtime",
            None,
            remote_schedule_count.metric_type(),
        )?;

        remote_schedule_count.encode(remote_schedule_count_encoder)?;

        let budget_forced_yield_count = ConstCounter::new(self.metrics.budget_forced_yield_count());

        let budget_forced_yield_count_encoder = encoder.encode_descriptor(
            "budget_forced_yield",
            "Returns the number of times that tasks have been forced to yield back to the scheduler after exhausting their task budgets",
            None,
            budget_forced_yield_count.metric_type(),
        )?;

        budget_forced_yield_count.encode(budget_forced_yield_count_encoder)?;

        let injection_queue_depth = ConstGauge::new(self.metrics.injection_queue_depth() as i64);

        let injection_queue_depth_encoder = encoder.encode_descriptor(
            "injection_queue_depth",
            "Returns the number of tasks currently scheduled in the runtime's injection queue",
            None,
            injection_queue_depth.metric_type(),
        )?;

        injection_queue_depth.encode(injection_queue_depth_encoder)?;

        let blocking_queue_depth = ConstGauge::new(self.metrics.blocking_queue_depth() as i64);

        let blocking_queue_depth_encoder = encoder.encode_descriptor(
            "blocking_queue_depth",
            "Returns the number of tasks currently scheduled in the blocking thread pool",
            None,
            blocking_queue_depth.metric_type(),
        )?;

        blocking_queue_depth.encode(blocking_queue_depth_encoder)?;

        for worker_id in 0..workers_count {
            let labels = [(WORKER_LABEL.to_string(), worker_id.to_string())];

            let worker_park = ConstCounter::new(self.metrics.worker_park_count(worker_id));

            let mut worker_park_encoder = encoder.encode_descriptor(
                "worker_park",
                "Returns the total number of times the given worker thread has parked",
                None,
                worker_park.metric_type(),
            )?;

            let worker_park_encoder = worker_park_encoder.encode_family(&labels)?;

            worker_park.encode(worker_park_encoder)?;

            let worker_noop_count = ConstCounter::new(self.metrics.worker_noop_count(worker_id));

            let mut worker_noop_count_encoder = encoder.encode_descriptor(
                "worker_noop_count",
                "Returns the number of times the given worker thread unparked but performed no work before parking again",
                None,
                worker_noop_count.metric_type(),
            )?;
            let worker_noop_count_encoder = worker_noop_count_encoder.encode_family(&labels)?;

            worker_noop_count.encode(worker_noop_count_encoder)?;

            let worker_steal = ConstCounter::new(self.metrics.worker_steal_count(worker_id));

            let mut worker_steal_encoder = encoder.encode_descriptor(
                "worker_steal",
                "Returns the number of tasks the given worker thread stole from another worker thread",
                None,
                worker_steal.metric_type(),
            )?;
            let worker_steal_encoder = worker_steal_encoder.encode_family(&labels)?;

            worker_steal.encode(worker_steal_encoder)?;

            let worker_steal_operations =
                ConstCounter::new(self.metrics.worker_steal_operations(worker_id));

            let mut worker_steal_operations_encoder = encoder.encode_descriptor(
                "worker_steal_operations",
                "Returns the number of times the given worker thread stole tasks from another worker thread",
                None,
                worker_steal_operations.metric_type(),
            )?;
            let worker_steal_operations_encoder =
                worker_steal_operations_encoder.encode_family(&labels)?;

            worker_steal_operations.encode(worker_steal_operations_encoder)?;

            let worker_poll = ConstCounter::new(self.metrics.worker_poll_count(worker_id));

            let mut worker_poll_encoder = encoder.encode_descriptor(
                "worker_poll",
                "Returns the number of tasks the given worker thread has polled",
                None,
                worker_poll.metric_type(),
            )?;
            let worker_poll_encoder = worker_poll_encoder.encode_family(&labels)?;

            worker_poll.encode(worker_poll_encoder)?;

            let worker_busy_duration = ConstCounter::new(
                self.metrics
                    .worker_total_busy_duration(worker_id)
                    .as_secs_f64(),
            );

            let mut worker_busy_duration_encoder = encoder.encode_descriptor(
                "worker_busy_duration_sec",
                "Returns the amount of time the given worker thread has been busy",
                None,
                worker_busy_duration.metric_type(),
            )?;

            let worker_busy_duration_encoder =
                worker_busy_duration_encoder.encode_family(&labels)?;

            worker_busy_duration.encode(worker_busy_duration_encoder)?;

            let worker_local_schedule =
                ConstCounter::new(self.metrics.worker_local_schedule_count(worker_id));

            let mut worker_local_schedule_encoder = encoder
                .encode_descriptor(
                    "worker_local_schedule",
                    "Returns the number of tasks scheduled from **within** the runtime on the given worker's local queue",
                    None,
                    worker_local_schedule.metric_type(),
                )?;
            let worker_local_schedule_encoder =
                worker_local_schedule_encoder.encode_family(&labels)?;

            worker_local_schedule.encode(worker_local_schedule_encoder)?;

            let worker_local_queue_depth =
                ConstGauge::new(self.metrics.worker_local_queue_depth(worker_id) as i64);

            let mut worker_local_queue_depth_encoder = encoder.encode_descriptor(
                "worker_local_queue_depth",
                "Returns the number of tasks currently scheduled in the given worker's local queue",
                None,
                worker_local_queue_depth.metric_type(),
            )?;
            let worker_local_queue_depth_encoder =
                worker_local_queue_depth_encoder.encode_family(&labels)?;

            worker_local_queue_depth.encode(worker_local_queue_depth_encoder)?;

            let worker_overflow = ConstCounter::new(self.metrics.worker_overflow_count(worker_id));

            let mut worker_overflow_encoder = encoder.encode_descriptor(
                "worker_overflow",
                "Returns the number of times the given worker thread saturated its local queue",
                None,
                worker_overflow.metric_type(),
            )?;
            let worker_overflow_encoder = worker_overflow_encoder.encode_family(&labels)?;

            worker_overflow.encode(worker_overflow_encoder)?;

            let worker_mean_poll_time =
                ConstCounter::new(self.metrics.worker_mean_poll_time(worker_id).as_secs_f64());

            let mut worker_mean_poll_time_encoder = encoder.encode_descriptor(
                "worker_mean_poll_time_sec",
                "Returns the mean duration of task polls",
                None,
                worker_overflow.metric_type(),
            )?;
            let worker_mean_poll_time_encoder =
                worker_mean_poll_time_encoder.encode_family(&labels)?;

            worker_mean_poll_time.encode(worker_mean_poll_time_encoder)?;

            if let Some(histogram_buckets) = histogram_buckets.as_ref() {
                let mut data: Vec<(f64, u64)> = Vec::with_capacity(histogram_buckets.0.len());
                let mut count: u64 = 0;

                for (bucket_id, bucket) in histogram_buckets.0.iter().enumerate() {
                    let mut key: f64 = bucket.end.as_nanos() as f64;
                    let value = self
                        .metrics
                        .poll_count_histogram_bucket_count(worker_id, bucket_id);
                    count += value;
                    if bucket_id == histogram_buckets.0.len() - 1 {
                        key = f64::MAX
                    }
                    data.push((key, value))
                }

                let histogram = TokioWorkerHistogram {
                    sum: 0f64,
                    count,
                    data,
                };

                let mut poll_count_histogram_encoder = encoder.encode_descriptor(
                    "poll_count_histogram",
                    "Returns the distribution of task poll times in nanoseconds",
                    None,
                    worker_overflow.metric_type(),
                )?;
                let poll_count_histogram_encoder =
                    poll_count_histogram_encoder.encode_family(&labels)?;

                histogram.encode(poll_count_histogram_encoder)?;
            }
        }

        Ok(())
    }
}
