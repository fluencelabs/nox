/*
 * Copyright 2021 Fluence Labs Limited
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

use particle_protocol::Particle;
use std::ops::Mul;

#[derive(Debug, Clone)]
pub struct Deadline {
    // Unix timestamp in milliseconds
    timestamp: u64,
    // TTL in milliseconds
    ttl: u32,
}

impl Deadline {
    pub fn from(particle: &Particle) -> Self {
        Self {
            timestamp: particle.timestamp,
            ttl: particle.ttl,
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.timestamp
            .checked_add(self.ttl as u64)
            // Whether ts is in the past
            .map(|ts| ts < now_ms)
            // If timestamp + ttl gives overflow, consider particle expired
            .unwrap_or_else(|| {
                log::warn!("timestamp {} + ttl {} overflowed", self.timestamp, self.ttl);
                true
            })
    }
}
