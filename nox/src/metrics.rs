use once_cell::sync::Lazy;
use prometheus_client::collector::Collector;
use prometheus_client::encoding::{EncodeMetric, MetricEncoder};
use prometheus_client::metrics::counter::ConstCounter;
use prometheus_client::metrics::gauge::ConstGauge;
use prometheus_client::metrics::MetricType;
use prometheus_client::registry::{Descriptor, LocalMetric, Prefix};
use prometheus_client::MaybeOwned;
use std::borrow::Cow;
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

static PREFIX: Lazy<Prefix> = Lazy::new(|| Prefix::from("tokio".to_string()));

static NUM_WORKERS_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "workers",
        "The number of worker threads used by the runtime",
        None,
        Some(&PREFIX),
        vec![],
    )
});

static ACTIVE_TASKS_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "active_tasks",
        "The number of active tasks in the runtime",
        None,
        Some(&PREFIX),
        vec![],
    )
});
static NUM_BLOCKING_THREADS_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "num_blocking_threads",
        "Еhe number of additional blocking threads spawned by the runtime",
        None,
        Some(&PREFIX),
        vec![],
    )
});

static NUM_IDLE_BLOCKING_THREADS_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "num_idle_blocking_threads",
        "Returns the number of idle blocking threads, which have spawned by the runtime",
        None,
        Some(&PREFIX),
        vec![],
    )
});
static REMOTE_SCHEDULE_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "remote_schedule",
        "Returns the number of tasks scheduled from outside of the runtime",
        None,
        Some(&PREFIX),
        vec![],
    )
});
static BUDGET_FORCED_YIELD_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "budget_forced_yield",
        "Returns the number of times that tasks have been forced to yield back to the scheduler after exhausting their task budgets",
        None,
        Some(&PREFIX),
        vec![],
    )
});

static INJECTION_QUEUE_DEPTH_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "injection_queue_depth",
        "Returns the number of tasks currently scheduled in the runtime's injection queue",
        None,
        Some(&PREFIX),
        vec![],
    )
});
static BLOCKING_QUEUE_DEPTH_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "blocking_queue_depth",
        "Returns the number of tasks currently scheduled in the blocking thread pool",
        None,
        Some(&PREFIX),
        vec![],
    )
});

const WORKER_LABEL: &str = "worker";

#[derive(Debug)]
struct TokioWorkerHistogram {
    sum: f64,
    count: u64,
    data: Vec<(f64, u64)>,
}

impl EncodeMetric for TokioWorkerHistogram {
    fn encode(&self, mut encoder: MetricEncoder<'_, '_>) -> std::result::Result<(), Error> {
        encoder.encode_histogram::<()>(self.sum, self.count, &self.data, None)
    }

    fn metric_type(&self) -> MetricType {
        MetricType::Histogram
    }
}
#[derive(Debug, Clone)]
struct TokioHistogramBuckets(Vec<Range<Duration>>);

type Result<'a> = (Cow<'a, Descriptor>, MaybeOwned<'a, Box<dyn LocalMetric>>);
impl Collector for TokioCollector {
    fn collect<'a>(&'a self) -> Box<dyn Iterator<Item = Result<'a>> + 'a> {
        let workers = self.metrics.num_workers();
        let histogram_buckets = if self.metrics.poll_count_histogram_enabled() {
            let num_buckets = self.metrics.poll_count_histogram_num_buckets();
            let bucket_ranges: Vec<Range<Duration>> = (0..num_buckets)
                .map(|index| self.metrics.poll_count_histogram_bucket_range(index))
                .collect();
            Some(TokioHistogramBuckets(bucket_ranges))
        } else {
            None
        };

        let mut result: Vec<Result<'a>> =
            Vec::with_capacity(8 + workers * (10 + histogram_buckets.as_ref().map_or(0, |_| 1))); //We preallocate a vector to reduce growing

        result.push((
            Cow::Borrowed(&*NUM_WORKERS_DESCRIPTOR),
            MaybeOwned::Owned(Box::new(ConstGauge::new(workers as i64))),
        ));

        result.push((
            Cow::Borrowed(&*NUM_BLOCKING_THREADS_DESCRIPTOR),
            MaybeOwned::Owned(Box::new(ConstGauge::new(
                self.metrics.num_blocking_threads() as i64,
            ))),
        ));

        result.push((
            Cow::Borrowed(&*ACTIVE_TASKS_DESCRIPTOR),
            MaybeOwned::Owned(Box::new(ConstGauge::new(
                self.metrics.active_tasks_count() as i64
            ))),
        ));
        result.push((
            Cow::Borrowed(&*NUM_IDLE_BLOCKING_THREADS_DESCRIPTOR),
            MaybeOwned::Owned(Box::new(ConstGauge::new(
                self.metrics.num_idle_blocking_threads() as i64,
            ))),
        ));
        result.push((
            Cow::Borrowed(&*REMOTE_SCHEDULE_DESCRIPTOR),
            MaybeOwned::Owned(Box::new(ConstCounter::new(
                self.metrics.remote_schedule_count(),
            ))),
        ));

        result.push((
            Cow::Borrowed(&*BUDGET_FORCED_YIELD_DESCRIPTOR),
            MaybeOwned::Owned(Box::new(ConstCounter::new(
                self.metrics.budget_forced_yield_count(),
            ))),
        ));

        result.push((
            Cow::Borrowed(&*INJECTION_QUEUE_DEPTH_DESCRIPTOR),
            MaybeOwned::Owned(Box::new(ConstGauge::new(
                self.metrics.injection_queue_depth() as i64,
            ))),
        ));
        result.push((
            Cow::Borrowed(&*BLOCKING_QUEUE_DEPTH_DESCRIPTOR),
            MaybeOwned::Owned(Box::new(ConstGauge::new(
                self.metrics.blocking_queue_depth() as i64,
            ))),
        ));

        for worker_id in 0..workers {
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_park",
                    "Returns the total number of times the given worker thread has parked",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstCounter::new(
                    self.metrics.worker_park_count(worker_id),
                ))),
            ));
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_noop",
                    "Returns the number of times the given worker thread unparked but performed no work before parking again",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstCounter::new(
                    self.metrics.worker_noop_count(worker_id),
                ))),
            ));
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_steal",
                    "Returns the number of tasks the given worker thread stole from another worker thread",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstCounter::new(
                    self.metrics.worker_steal_count(worker_id),
                ))),
            ));
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_steal_operations",
                    "Returns the number of times the given worker thread stole tasks from another worker thread",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstCounter::new(
                    self.metrics.worker_steal_operations(worker_id),
                ))),
            ));
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_poll",
                    "Returns the number of tasks the given worker thread has polled",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstCounter::new(
                    self.metrics.worker_poll_count(worker_id),
                ))),
            ));
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_busy_duration_msec",
                    "Returns the amount of time the given worker thread has been busy",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstCounter::new(
                    self.metrics
                        .worker_total_busy_duration(worker_id)
                        .as_millis() as u64,
                ))),
            ));
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_local_schedule",
                    "Returns the number of tasks scheduled from **within** the runtime on the given worker's local queue",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstCounter::new(
                    self.metrics
                        .worker_local_schedule_count(worker_id)
                ))),
            ));
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_local_queue_depth",
                    "Returns the number of tasks currently scheduled in the given worker's local queue",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstGauge::new(
                    self.metrics
                        .worker_local_queue_depth(worker_id) as i64
                ))),
            ));
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_overflow",
                    "Returns the number of times the given worker thread saturated its local queue",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstCounter::new(
                    self.metrics.worker_overflow_count(worker_id),
                ))),
            ));
            result.push((
                Cow::Owned(Descriptor::new(
                    "worker_mean_poll_time_msec",
                    "Returns the mean duration of task polls",
                    None,
                    Some(&PREFIX),
                    vec![(
                        Cow::Borrowed(WORKER_LABEL),
                        Cow::Owned(worker_id.to_string()),
                    )],
                )),
                MaybeOwned::Owned(Box::new(ConstCounter::new(
                    self.metrics.worker_mean_poll_time(worker_id).as_millis() as u64,
                ))),
            ));
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

                result.push((
                    Cow::Owned(Descriptor::new(
                        "poll_count_histogram",
                        "Returns the distribution of task poll times in nanoseconds",
                        None,
                        Some(&PREFIX),
                        vec![(
                            Cow::Borrowed(WORKER_LABEL),
                            Cow::Owned(worker_id.to_string()),
                        )],
                    )),
                    MaybeOwned::Owned(Box::new(histogram)),
                ));
            }
        }

        Box::new(result.into_iter())
    }
}
