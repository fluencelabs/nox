use humantime::FormattedDuration;

/// Function that logs for different builtin namespaces
pub fn builtin_log_fn(
    service: &str,
    args: String,
    elapsed: FormattedDuration,
    particle_id: String,
) {
    match service {
        "array" | "cmp" | "debug" | "math" | "op" | "getDataSrv" | "run-console" => {
            tracing::event!(
                tracing::Level::DEBUG,
                "Executed host call {} ({}) [{}]",
                args,
                elapsed,
                particle_id
            )
        }
        "peer" | "script" | "stat" | "sig" | "srv" | "dist" | "kad" => tracing::event!(
            tracing::Level::INFO,
            "Executed host call {} ({}) [{}]",
            args,
            elapsed,
            particle_id
        ),
        _ => tracing::event!(
            tracing::Level::INFO,
            "Executed host call {} ({}) [{}]",
            args,
            elapsed,
            particle_id
        ),
    }
}
