/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
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
