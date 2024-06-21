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

use particle_protocol::Particle;

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
