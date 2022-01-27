use open_metrics_client::metrics::counter::Counter;
use open_metrics_client::metrics::gauge::Gauge;
use open_metrics_client::registry::Registry;

#[derive(Clone)]
pub struct VmPoolMetrics {
    pub pool_size: Gauge,
    pub free_vms: Gauge,
    pub get_vm: Counter,
    pub put_vm: Counter,
    pub no_free_vm: Counter,
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

        Self {
            pool_size,
            free_vms,
            get_vm,
            put_vm,
            no_free_vm,
        }
    }
}
