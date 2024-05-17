use humantime::FormattedDuration;

/// Truncate string to be at max 500 graphemes
fn truncate(s: &str) -> &str {
    match s.char_indices().nth(500) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

/// Function that logs for different builtin namespaces
pub fn builtin_log_fn(service: &str, args: &str, elapsed: FormattedDuration, particle_id: String) {
    let args = truncate(args);
    match service {
        "run-console" => {
            tracing::event!(
                tracing::Level::INFO,
                "Executed host call {} ({}) [{}]",
                args,
                elapsed,
                particle_id
            )
        }
        "array" | "cmp" | "debug" | "math" | "op" | "getDataSrv" | "json" => {
            tracing::event!(
                tracing::Level::TRACE,
                "Executed host call {} ({}) [{}]",
                args,
                elapsed,
                particle_id
            )
        }
        "peer" | "stat" | "sig" | "srv" | "dist" | "kad" => tracing::event!(
            tracing::Level::DEBUG,
            "Executed host call {} ({}) [{}]",
            args,
            elapsed,
            particle_id
        ),
        _ => tracing::event!(
            tracing::Level::DEBUG,
            "Executed host call {} ({}) [{}]",
            args,
            elapsed,
            particle_id
        ),
    }
}
