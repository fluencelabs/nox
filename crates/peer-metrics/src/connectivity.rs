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

use crate::{ParticleLabel, ParticleType};
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;

#[derive(EncodeLabelValue, Hash, Clone, Eq, PartialEq, Debug)]
pub enum Resolution {
    Local,
    Kademlia,
    KademliaNotFound,
    KademliaError,
    ConnectionFailed,
}
#[derive(EncodeLabelSet, Hash, Clone, Eq, PartialEq, Debug)]
pub struct ResolutionLabel {
    action: Resolution,
}

#[derive(Clone)]
pub struct ConnectivityMetrics {
    contact_resolve: Family<ResolutionLabel, Counter>,
    pub particle_send_success: Family<ParticleLabel, Counter>,
    pub particle_send_failure: Family<ParticleLabel, Counter>,
    pub bootstrap_disconnected: Counter,
    pub bootstrap_connected: Counter,
}

impl ConnectivityMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("connectivity");

        let contact_resolve = Family::default();
        sub_registry.register(
            "contact_resolve",
            "Counters regarding contact resolution in particle processing",
            contact_resolve.clone(),
        );

        let particle_send_success = Family::default();
        sub_registry.register(
            "particle_send_success",
            "Number of sent particles",
            particle_send_success.clone(),
        );

        let particle_send_failure = Family::default();
        sub_registry.register(
            "particle_send_failure",
            "Number of errors on particle sending",
            particle_send_failure.clone(),
        );

        let bootstrap_disconnected = Counter::default();
        sub_registry.register(
            "bootstrap_disconnected",
            "Number of times peer disconnected from bootstrap peers",
            bootstrap_disconnected.clone(),
        );

        let bootstrap_connected = Counter::default();
        sub_registry.register(
            "bootstrap_connected",
            "Number of times peer connected (or reconnected) to a bootstrap peer",
            bootstrap_connected.clone(),
        );

        Self {
            contact_resolve,
            particle_send_success,
            particle_send_failure,
            bootstrap_disconnected,
            bootstrap_connected,
        }
    }

    pub fn count_resolution(&self, resolution: Resolution) {
        self.contact_resolve
            .get_or_create(&ResolutionLabel { action: resolution })
            .inc();
    }

    pub fn send_particle_ok(&self, particle: &str) {
        self.particle_send_success
            .get_or_create(&ParticleLabel {
                particle_type: ParticleType::from_particle(particle),
            })
            .inc();
    }

    pub fn send_particle_failed(&self, particle: &str) {
        self.particle_send_failure
            .get_or_create(&ParticleLabel {
                particle_type: ParticleType::from_particle(particle),
            })
            .inc();
    }
}
