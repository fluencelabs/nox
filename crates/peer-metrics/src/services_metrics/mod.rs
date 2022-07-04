pub mod builtin;
pub mod instant;

pub use crate::services_metrics::builtin::{ServicesMetricsBuiltin, Observation};
pub use crate::services_metrics::instant::{
    ServicesMetrics as ServicesMetricsInstant, ServicesMetricsBackend,
};

use std::fmt;

#[derive(Clone)]
pub struct ServicesMetrics {
    pub instant: Option<ServicesMetricsInstant>,
    pub builtin: ServicesMetricsBuiltin,
}

impl fmt::Debug for ServicesMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServicesMetrics").finish()
    }
}

impl ServicesMetrics {
    pub fn new(instant: Option<ServicesMetricsInstant>) -> Self {
        ServicesMetrics {
            instant,
            builtin: ServicesMetricsBuiltin::new(),
        }
    }
}
