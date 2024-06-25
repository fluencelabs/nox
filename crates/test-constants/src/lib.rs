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
#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns,
    unreachable_code
)]

use std::time::Duration;

/// In debug, VM startup time is big, account for that
#[cfg(debug_assertions)]
pub static TIMEOUT: Duration = Duration::from_secs(150);
#[cfg(not(debug_assertions))]
pub static TIMEOUT: Duration = Duration::from_secs(15);

pub static SHORT_TIMEOUT: Duration = Duration::from_millis(300);
pub static TRANSPORT_TIMEOUT: Duration = Duration::from_millis(500);
pub static IDLE_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
pub static EXECUTION_TIMEOUT: Duration = Duration::from_millis(5000);
pub static PARTICLE_TTL: u32 = 20000;
