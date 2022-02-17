use std::cmp::{max, min};

use bytesize::MIB;
use open_metrics_client::metrics::counter::Counter;
use open_metrics_client::metrics::gauge::Gauge;
use open_metrics_client::metrics::histogram::Histogram;
use open_metrics_client::registry::Registry;

#[derive(Clone)]
pub struct VmPoolMetrics {
    pool_size: Gauge,
    pub free_vms: Gauge,
    pub get_vm: Counter,
    pub put_vm: Counter,
    pub no_free_vm: Counter,

    pub vm_mem_max_value: u64,
    pub vm_mem_max: Gauge,
    pub vm_mem_min_value: u64,
    pub vm_mem_min: Gauge,
    // store memory sizes for each vm
    pub vm_mems: Vec<u64>,
    pub vm_mem_total: Gauge,
    // cumulative moving average
    pub vm_mem_cma: u64,
    pub vm_mem_measures: u64,
    pub vm_mem_avg: Gauge,
    // histogram
    pub vm_mem_histo: Histogram,
}

impl VmPoolMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("aqua_vm_pool");

        let pool_size = Gauge::default();
        sub_registry.register(
            "pool_size",
            "Size of the AquaVM pool",
            Box::new(pool_size.clone()),
        );

        let free_vms = Gauge::default();
        sub_registry.register(
            "free_vms",
            "Number of currently free AquaVMs",
            Box::new(free_vms.clone()),
        );

        let get_vm = Counter::default();
        sub_registry.register(
            "get_vm",
            "Number of times an AquaVM has been taken from the pool",
            Box::new(get_vm.clone()),
        );

        let put_vm = Counter::default();
        sub_registry.register(
            "put_vm",
            "Number of times an AquaVM has been put back to the pool",
            Box::new(put_vm.clone()),
        );

        let no_free_vm = Counter::default();
        sub_registry.register(
            "no_free_vm",
            "Number of time when we tried to take an AquaVM from an empty pool",
            Box::new(no_free_vm.clone()),
        );

        let vm_mem_max = Gauge::default();
        sub_registry.register(
            "vm_mem_max",
            "Maximum allocated memory among all interpreters (after first interpretation)",
            Box::new(vm_mem_max.clone()),
        );
        let vm_mem_min = Gauge::default();
        sub_registry.register(
            "vm_mem_min",
            "Minumum allocated memory among all interpreters (after first interpretation)",
            Box::new(vm_mem_min.clone()),
        );
        let vm_mem_total = Gauge::default();
        sub_registry.register(
            "vm_mem_total",
            "Total memory allocated by all interpreters on node",
            Box::new(vm_mem_total.clone()),
        );
        let vm_mem_avg = Gauge::default();
        sub_registry.register(
            "vm_mem_avg",
            "Average allocated memory of an interpreter",
            Box::new(vm_mem_avg.clone()),
        );
        // 1mb, 5mb, 10mb, 25mb, 50mb, 100mb, 200mb
        let vm_mem_histo = Histogram::new(
            vec![1, 5, 10, 25, 50, 100, 200]
                .into_iter()
                .map(|n| (n * MIB) as f64),
        );
        sub_registry.register(
            "vm_mem_histo",
            "Interpreter memory size distribution",
            Box::new(vm_mem_histo.clone()),
        );

        Self {
            pool_size,
            free_vms,
            get_vm,
            put_vm,
            no_free_vm,

            vm_mem_max_value: 0,
            vm_mem_max,
            vm_mem_min_value: 0,
            vm_mem_min,
            vm_mems: vec![],
            vm_mem_total,
            vm_mem_cma: 0,
            vm_mem_measures: 0,
            vm_mem_avg,
            vm_mem_histo,
        }
    }

    pub fn set_pool_size(&mut self, size: usize) {
        self.pool_size.set(size as u64);
        self.vm_mems.resize(size, 0);
    }

    pub fn measure_memory(&mut self, idx: usize, memory_size: u64) {
        // TODO: this is a HACK until we stop using `get_vm` for cleaning up Actor resources.
        //       Until then, intentionally ignore memory measurements for AquaVMs that haven't
        //       yet processed any particles.
        if memory_size == 0 {
            return;
        }

        // Histogram
        self.vm_mem_histo.observe(memory_size as f64);

        // Cumulative Moving Average
        // cma_n+1 = cma_n + ((x_n+1 - cma_n) / (n + 1))
        // https://en.wikipedia.org/wiki/Moving_average#Cumulative_moving_average
        self.vm_mem_measures += 1;
        let cma = self.vm_mem_cma as i64;
        let next_cma = cma + ((memory_size as i64 - cma) / self.vm_mem_measures as i64);
        self.vm_mem_cma = next_cma.abs() as u64;
        self.vm_mem_avg.set(self.vm_mem_cma);

        // Max mem
        self.vm_mem_max_value = max(self.vm_mem_max_value, memory_size);
        self.vm_mem_max.set(self.vm_mem_max_value);

        // Min mem
        self.vm_mem_min_value = min(self.vm_mem_min_value, memory_size);
        self.vm_mem_min.set(self.vm_mem_min_value);

        // Total
        debug_assert!(idx < self.vm_mems.len());
        if let Some(prev) = self.vm_mems.get_mut(idx) {
            if *prev != memory_size {
                *prev = memory_size;
                let total = self.vm_mems.iter().sum();
                self.vm_mem_total.set(total);
            }
        } else {
            log::error!(
                "unexpected: measure_memory idx {} is greater than pool size {}",
                idx,
                self.vm_mems.len()
            );
        }
    }
}
