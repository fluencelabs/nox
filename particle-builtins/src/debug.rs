use particle_execution::ServiceFunction;
use std::collections::HashMap;

pub fn fmt_custom_services(
    services: &HashMap<String, HashMap<String, ServiceFunction>>,
    fmt: &mut std::fmt::Formatter<'_>,
) -> Result<(), std::fmt::Error> {
    fmt.debug_map()
        .entries(
            services
                .iter()
                .map(|(sid, fs)| (sid, fs.iter().map(|(fname, _)| fname).collect::<Vec<_>>())),
        )
        .finish()
}
