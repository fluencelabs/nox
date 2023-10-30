use once_cell::sync::Lazy;
use prometheus_client::collector::Collector;
use prometheus_client::metrics::counter::ConstCounter;
use prometheus_client::metrics::gauge::ConstGauge;
use prometheus_client::registry::{Descriptor, LocalMetric, Prefix};
use prometheus_client::MaybeOwned;
use std::borrow::Cow;
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
        "workers_count",
        "The number of worker threads used by the runtime",
        None,
        Some(&PREFIX),
        vec![],
    )
});

static ACTIVE_TASKS_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "active_tasks_count",
        "The number of active tasks in the runtime",
        None,
        Some(&PREFIX),
        vec![],
    )
});
static NUM_BLOCKING_THREADS_DESCRIPTOR: Lazy<Descriptor> = Lazy::new(|| {
    Descriptor::new(
        "num_blocking_threads",
        "Еhe number of additional blocking threads spawned by the runtime.",
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
        "remote_schedule_count",
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

impl Collector for TokioCollector {
    fn collect<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (Cow<'a, Descriptor>, MaybeOwned<'a, Box<dyn LocalMetric>>)> + 'a>
    {
        let num_workers: Box<dyn LocalMetric> =
            Box::new(ConstGauge::new(self.metrics.num_workers() as i64));

        let active_tasks_count: Box<dyn LocalMetric> =
            Box::new(ConstGauge::new(self.metrics.active_tasks_count() as i64));

        let num_blocking_threads: Box<dyn LocalMetric> =
            Box::new(ConstGauge::new(self.metrics.num_blocking_threads() as i64));

        let num_idle_blocking_threads: Box<dyn LocalMetric> = Box::new(ConstGauge::new(
            self.metrics.num_idle_blocking_threads() as i64,
        ));

        let remote_schedule_count: Box<dyn LocalMetric> =
            Box::new(ConstCounter::new(self.metrics.remote_schedule_count()));

        let budget_forced_yield_count: Box<dyn LocalMetric> =
            Box::new(ConstCounter::new(self.metrics.budget_forced_yield_count()));

        let injection_queue_depth: Box<dyn LocalMetric> =
            Box::new(ConstGauge::new(self.metrics.injection_queue_depth() as i64));

        let blocking_queue_depth: Box<dyn LocalMetric> =
            Box::new(ConstGauge::new(self.metrics.blocking_queue_depth() as i64));

        Box::new(
            [
                (
                    Cow::Borrowed(&*NUM_WORKERS_DESCRIPTOR),
                    MaybeOwned::Owned(num_workers),
                ),
                (
                    Cow::Borrowed(&*NUM_BLOCKING_THREADS_DESCRIPTOR),
                    MaybeOwned::Owned(num_blocking_threads),
                ),
                (
                    Cow::Borrowed(&*ACTIVE_TASKS_DESCRIPTOR),
                    MaybeOwned::Owned(active_tasks_count),
                ),
                (
                    Cow::Borrowed(&*NUM_IDLE_BLOCKING_THREADS_DESCRIPTOR),
                    MaybeOwned::Owned(num_idle_blocking_threads),
                ),
                (
                    Cow::Borrowed(&*REMOTE_SCHEDULE_DESCRIPTOR),
                    MaybeOwned::Owned(remote_schedule_count),
                ),
                (
                    Cow::Borrowed(&*BUDGET_FORCED_YIELD_DESCRIPTOR),
                    MaybeOwned::Owned(budget_forced_yield_count),
                ),
                (
                    Cow::Borrowed(&*INJECTION_QUEUE_DEPTH_DESCRIPTOR),
                    MaybeOwned::Owned(injection_queue_depth),
                ),
                (
                    Cow::Borrowed(&*BLOCKING_QUEUE_DEPTH_DESCRIPTOR),
                    MaybeOwned::Owned(blocking_queue_depth),
                ),
            ]
            .into_iter(),
        )
    }
}
