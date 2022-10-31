/*
 * Copyright 2020 Fluence Labs Limited
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
