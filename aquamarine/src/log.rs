/*
 * Copyright 2024 Fluence DAO
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

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
        "array" | "cmp" | "debug" | "math" | "op" | "getDataSrv" | "run-console" | "json" => {
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
