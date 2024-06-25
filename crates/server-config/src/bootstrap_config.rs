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

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    #[serde(with = "humantime_serde")]
    pub reconnect_delay: Duration,
    #[serde(with = "humantime_serde")]
    pub bootstrap_delay: Duration,
    #[serde(with = "humantime_serde")]
    pub bootstrap_max_delay: Duration,
}

impl BootstrapConfig {
    /// Creates config with all values to zero, so no delays. Useful for tests.
    pub fn zero() -> BootstrapConfig {
        BootstrapConfig {
            reconnect_delay: <_>::default(),
            bootstrap_delay: <_>::default(),
            bootstrap_max_delay: <_>::default(),
        }
    }
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        use rand::prelude::*;
        let mut rng = rand::thread_rng();
        BootstrapConfig {
            // TODO: make it exponential
            reconnect_delay: Duration::from_millis(1500 + rng.gen_range(0..500)),
            bootstrap_delay: Duration::from_millis(30000 + rng.gen_range(0..2000)),
            bootstrap_max_delay: Duration::from_secs(60),
        }
    }
}
