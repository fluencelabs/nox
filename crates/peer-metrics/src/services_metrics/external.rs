/*
 * Copyright 2024 Fluence DAO
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/**
 * Services metrics that are meant to be written to an external metrics storage like Prometheus
 */
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{linear_buckets, Histogram};
use prometheus_client::registry::Registry;
use std::fmt::Write;

use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue, LabelValueEncoder};
use prometheus_client::metrics::family::Family;

use crate::{execution_time_buckets, mem_buckets_4gib, mem_buckets_8gib, register};

#[derive(Hash, Clone, Eq, PartialEq, Debug)]
pub enum ServiceType {
    Builtin,
    Spell(Option<String>),
    Service(Option<String>),
}

impl EncodeLabelValue for ServiceType {
    fn encode(&self, encoder: &mut LabelValueEncoder) -> Result<(), std::fmt::Error> {
        let label = match self {
            ServiceType::Builtin => "builtin",
            ServiceType::Spell(Some(x)) => x,
            ServiceType::Spell(_) => "spell",
            ServiceType::Service(Some(x)) => x,
            ServiceType::Service(_) => "non-aliased-services",
        };
        encoder.write_str(label)?;
        Ok(())
    }
}

#[derive(EncodeLabelSet, Hash, Clone, Eq, PartialEq, Debug)]
pub struct ServiceTypeLabel {
    pub service_type: ServiceType,
}

#[derive(Clone)]
pub struct ServicesMemoryMetrics {
    /// Actual memory used by a module
    pub mem_max_per_module_bytes: Histogram,
    /// Actual memory used by a service
    pub mem_used_bytes: Family<ServiceTypeLabel, Histogram>,
    /// Actual memory used by a module
    pub mem_used_per_module_bytes: Family<ServiceTypeLabel, Histogram>,
    /// Total memory used
    pub mem_used_total_bytes: Family<ServiceTypeLabel, Gauge>,
}

#[derive(Clone)]
pub struct ServicesMetricsExternal {
    /// Number of currently running services
    pub services_count: Family<ServiceTypeLabel, Gauge>,
    /// How long it took to create a service
    pub creation_time_msec: Family<ServiceTypeLabel, Histogram>,
    /// How long it took to remove a service
    pub removal_time_msec: Family<ServiceTypeLabel, Histogram>,
    /// Number of (srv create) calls
    pub creation_count: Family<ServiceTypeLabel, Counter>,
    /// Number of (srv remove) calls
    pub removal_count: Family<ServiceTypeLabel, Counter>,

    /// Number of (srv create) failures
    pub creation_failure_count: Counter,

    /// How many modules a service includes.
    pub modules_in_services_count: Histogram,

    /// Service call time
    pub call_time_sec: Family<ServiceTypeLabel, Histogram>,
    pub lock_wait_time_sec: Family<ServiceTypeLabel, Histogram>,
    pub call_success_count: Family<ServiceTypeLabel, Counter>,
    pub call_failed_count: Family<ServiceTypeLabel, Counter>,

    /// Memory metrics
    pub memory_metrics: ServicesMemoryMetrics,
}

impl ServicesMetricsExternal {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("services");

        let services_count: Family<_, _> = register(
            sub_registry,
            Family::new_with_constructor(Gauge::default),
            "services_count",
            "number of currently running services",
        );

        let creation_time_msec: Family<_, _> = register(
            sub_registry,
            Family::new_with_constructor(|| Histogram::new(execution_time_buckets())),
            "creation_time_msec",
            "how long it took to create a service",
        );

        let removal_time_msec: Family<_, _> = register(
            sub_registry,
            Family::new_with_constructor(|| Histogram::new(execution_time_buckets())),
            "removal_time_msec",
            "how long it took to remove a service",
        );

        let creation_count: Family<_, _> = register(
            sub_registry,
            Family::new_with_constructor(Counter::default),
            "creation_count",
            "number of srv create calls",
        );

        let removal_count: Family<_, _> = register(
            sub_registry,
            Family::new_with_constructor(Counter::default),
            "removal_count",
            "number of srv remove calls",
        );

        let mem_max_per_module_bytes = register(
            sub_registry,
            Histogram::new(mem_buckets_4gib()),
            "mem_max_per_module_bytes",
            "maximum memory set in module config",
        );

        let mem_used_bytes: Family<_, _> = register(
            sub_registry,
            Family::new_with_constructor(|| Histogram::new(mem_buckets_8gib())),
            "mem_used_bytes",
            "actual memory used by a service",
        );

        let mem_used_per_module_bytes: Family<_, _> = register(
            sub_registry,
            Family::new_with_constructor(|| Histogram::new(mem_buckets_4gib())),
            "mem_used_per_module_bytes",
            "actual memory used by a service per module",
        );

        let mem_used_total_bytes = register(
            sub_registry,
            Family::default(),
            "mem_used_total_bytes",
            "total size of used memory by services",
        );

        let creation_failure_count = register(
            sub_registry,
            Counter::default(),
            "creation_failure_count",
            "number of srv remove calls",
        );

        let modules_in_services_count = register(
            sub_registry,
            Histogram::new(linear_buckets(1.0, 1.0, 10)),
            "modules_in_services_count",
            "number of modules per services",
        );

        let call_time_sec: Family<_, _> = register(
            sub_registry,
            Family::new_with_constructor(|| Histogram::new(execution_time_buckets())),
            "call_time_msec",
            "how long it took to execute a call",
        );

        let lock_wait_time_sec: Family<_, _> = register(
            sub_registry,
            Family::new_with_constructor(|| Histogram::new(execution_time_buckets())),
            "lock_wait_time_sec",
            "how long a service waited for Mutex",
        );

        let memory_metrics = ServicesMemoryMetrics {
            mem_max_per_module_bytes,
            mem_used_bytes,
            mem_used_per_module_bytes,
            mem_used_total_bytes,
        };
        let call_success_count = register(
            sub_registry,
            Family::default(),
            "call_success_count",
            "count of successfully executed calls",
        );

        let call_failed_count = register(
            sub_registry,
            Family::default(),
            "call_failed_count",
            "count of fails of calls execution",
        );
        Self {
            services_count,
            creation_time_msec,
            removal_time_msec,
            creation_count,
            removal_count,
            creation_failure_count,
            modules_in_services_count,
            call_time_sec,
            lock_wait_time_sec,
            call_success_count,
            call_failed_count,
            memory_metrics,
        }
    }

    /// Collect all metrics that are relevant on service removal.
    pub fn observe_removed(&self, service_type: ServiceType, removal_time: f64) {
        let label = ServiceTypeLabel { service_type };
        self.removal_count.get_or_create(&label).inc();
        self.services_count.get_or_create(&label).dec();
        self.removal_time_msec
            .get_or_create(&label)
            .observe(removal_time);
    }

    pub fn observe_created(&self, service_type: ServiceType, modules_num: f64, creation_time: f64) {
        let label = ServiceTypeLabel { service_type };
        self.services_count.get_or_create(&label).inc();
        self.modules_in_services_count.observe(modules_num);
        self.creation_count.get_or_create(&label).inc();
        self.creation_time_msec
            .get_or_create(&label)
            .observe(creation_time);
    }
}
