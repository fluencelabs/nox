use crate::register;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;

#[derive(Clone)]
pub struct SpellMetrics {
    spell_particles_created: Counter,
    spell_scheduled_now: Gauge,
    spell_periods: Histogram,
}

impl SpellMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("spell");

        let spell_particles_created = register(
            sub_registry,
            Counter::default(),
            "particles_created",
            "Number of spell particles created",
        );

        let spell_scheduled_now = register(
            sub_registry,
            Gauge::default(),
            "scheduled_now",
            "Number of spell particles scheduled to run now",
        );

        let spell_periods = register(
            sub_registry,
            Histogram::new(Self::periods_buckets()),
            "periods",
            "Spell particle periods",
        );

        Self {
            spell_particles_created,
            spell_scheduled_now,
            spell_periods,
        }
    }

    fn periods_buckets() -> std::vec::IntoIter<f64> {
        // 1 sec, 30 sec, 1 min, 5 min, 10 min,  1 hour, 12 hours, 1 day, 1 week, 1 month
        vec![
            1.0,
            30.0,
            60.0,
            60.0 * 5.0,
            60.0 * 10.0,
            60.0 * 60.0,
            60.0 * 60.0 * 12.0,
            60.0 * 60.0 * 24.0,
            60.0 * 60.0 * 24.0 * 7.0,
            60.0 * 60.0 * 24.0 * 30.0,
        ]
        .into_iter()
    }

    pub fn observe_started_spell(&self, period: u32) {
        self.spell_scheduled_now.inc();
        self.spell_periods.observe(period as f64)
    }

    pub fn observe_finished_spell(&self) {
        self.spell_scheduled_now.dec();
    }

    pub fn observe_spell_cast(&self) {
        self.spell_particles_created.inc();
    }
}
