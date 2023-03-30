/// Function that prescribes different log levels for different builtin namespaces
pub fn builtin_log_level(service: &str) -> log::Level {
    match service {
        "array" | "cmp" | "debug" | "math" | "op" | "getDataSrv" | "run-console" => {
            log::Level::Debug
        }
        "peer" | "script" | "stat" | "sig" | "srv" | "dist" | "kad" => log::Level::Info,
        _ => log::Level::Info,
    }
}
