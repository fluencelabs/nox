use std::collections::HashMap;

use parking_lot::{Mutex, RwLock};

use particle_execution::ServiceFunction;

pub fn fmt_custom_services(
    services: &RwLock<HashMap<String, HashMap<String, Mutex<ServiceFunction>>>>,
    fmt: &mut std::fmt::Formatter<'_>,
) -> Result<(), std::fmt::Error> {
    fmt.debug_map()
        .entries(
            services
                .read()
                .iter()
                .map(|(sid, fs)| (sid, fs.iter().map(|(fname, _)| fname).collect::<Vec<_>>())),
        )
        .finish()
}
