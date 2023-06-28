use humantime::FormattedDuration;

/// Function that prescribes different log levels for different builtin namespaces
pub fn builtin_log_level(service: &str) -> fn((String, FormattedDuration, String)) -> () {
    match service {
        "array" | "cmp" | "debug" | "math" | "op" | "getDataSrv" | "run-console" => {
            |(args, elapsed, particle_id)| {
                tracing::event!(
                    tracing::Level::DEBUG,
                    "Executed host call {} ({}) [{}]",
                    args,
                    elapsed,
                    particle_id
                );
            }
        }
        "peer" | "script" | "stat" | "sig" | "srv" | "dist" | "kad" => {
            |(args, elapsed, particle_id)| {
                tracing::event!(
                    tracing::Level::INFO,
                    "Executed host call {} ({}) [{}]",
                    args,
                    elapsed,
                    particle_id
                );
            }
        }
        _ => |(args, elapsed, particle_id)| {
            tracing::event!(
                tracing::Level::INFO,
                "Executed host call {} ({}) [{}]",
                args,
                elapsed,
                particle_id
            );
        },
    }
}
