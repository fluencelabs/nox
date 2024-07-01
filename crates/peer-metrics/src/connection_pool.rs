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

use crate::{ParticleLabel, ParticleType};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::{exponential_buckets, Histogram};
use prometheus_client::registry::Registry;

#[derive(Clone)]
pub struct ConnectionPoolMetrics {
    pub received_particles: Family<ParticleLabel, Counter>,
    pub particle_sizes: Family<ParticleLabel, Histogram>,
    pub connected_peers: Gauge,
    pub particle_queue_size: Gauge,
}

impl ConnectionPoolMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("connection_pool");

        let received_particles = Family::default();
        sub_registry.register(
            "received_particles",
            "Number of particles received from the network (not unique)",
            received_particles.clone(),
        );

        // from 100 bytes to 100 MB
        let particle_sizes: Family<_, _> =
            Family::new_with_constructor(|| Histogram::new(exponential_buckets(100.0, 10.0, 7)));
        sub_registry.register(
            "particle_sizes",
            "Distribution of particle data sizes",
            particle_sizes.clone(),
        );

        let connected_peers = Gauge::default();
        sub_registry.register(
            "connected_peers",
            "Number of peers we have connections to at a given moment",
            connected_peers.clone(),
        );

        let particle_queue_size = Gauge::default();
        sub_registry.register(
            "particle_queue_size",
            "Size of a particle queue in connection pool",
            particle_queue_size.clone(),
        );

        Self {
            received_particles,
            particle_sizes,
            connected_peers,
            particle_queue_size,
        }
    }

    pub fn incoming_particle(&self, particle_id: &str, queue_len: i64, particle_len: f64) {
        self.particle_queue_size.set(queue_len);
        let label = ParticleLabel {
            particle_type: ParticleType::from_particle(particle_id),
        };
        self.received_particles.get_or_create(&label).inc();
        self.particle_sizes
            .get_or_create(&label)
            .observe(particle_len);
    }
}
