use humantime::FormattedDuration;

/// Function that logs for different builtin namespaces
pub fn builtin_log_fn(service: &str) -> fn((String, FormattedDuration, String)) -> () {
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
