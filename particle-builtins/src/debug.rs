use std::collections::HashMap;

use parking_lot::RwLock;

use crate::builtins::CustomService;

pub fn fmt_custom_services(
    services: &RwLock<HashMap<String, CustomService>>,
    fmt: &mut std::fmt::Formatter<'_>,
) -> Result<(), std::fmt::Error> {
    fmt.debug_map()
        .entries(
            services
                .read()
                .iter()
                .map(|(sid, fs)| (sid, fs.functions.keys().collect::<Vec<_>>())),
        )
        .finish()
}
