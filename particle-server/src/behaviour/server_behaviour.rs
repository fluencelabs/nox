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

use crate::bootstrapper::Bootstrapper;
use crate::config::BehaviourConfig;
use anyhow::Context;
use fluence_libp2p::{event_polling, generate_swarm_event_type};
use libp2p::core::Multiaddr;
use libp2p::{
    identify::Identify,
    identity::PublicKey,
    ping::{Ping, PingConfig, PingEvent},
};
use particle_behaviour::{ParticleBehaviour, ParticleConfig};
use std::collections::VecDeque;

pub type SwarmEventType = generate_swarm_event_type!(ServerBehaviour);

/// Coordinates protocols, so they can cooperate
#[derive(::libp2p::NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll")]
pub struct ServerBehaviour {
    bootstrapper: Bootstrapper,
    identity: Identify,
    ping: Ping,
    pub(super) particle: ParticleBehaviour,
    #[behaviour(ignore)]
    events: VecDeque<SwarmEventType>,
}

impl ServerBehaviour {
    pub fn new(cfg: BehaviourConfig<'_>) -> anyhow::Result<Self> {
        let local_public_key = PublicKey::Ed25519(cfg.key_pair.public());
        let identity = Identify::new(
            "/fluence/faas/1.0.0".into(),
            "0.1.0".into(),
            local_public_key,
        );
        let ping = Ping::new(PingConfig::new().with_keep_alive(false));

        let config = ParticleConfig::new(
            cfg.protocol_config,
            cfg.local_peer_id.clone(),
            cfg.services_base_dir,
            cfg.services_envs,
            cfg.stepper_base_dir,
            cfg.air_interpreter,
            cfg.key_pair,
            cfg.stepper_pool_size,
            cfg.external_addresses,
        );
        let particle = ParticleBehaviour::new(config, cfg.trust_graph, cfg.registry)
            .context("failed to create ParticleBehvaiour")?;
        let bootstrapper = Bootstrapper::new(cfg.bootstrap, cfg.local_peer_id, cfg.bootstrap_nodes);

        Ok(Self {
            identity,
            ping,
            particle,
            bootstrapper,
            events: <_>::default(),
        })
    }

    /// Dials bootstrap nodes
    pub fn dial_bootstrap_nodes(&mut self) {
        // TODO: how to avoid collect?
        let bootstrap_nodes: Vec<_> = self.bootstrapper.bootstrap_nodes.iter().cloned().collect();
        if bootstrap_nodes.is_empty() {
            log::warn!("No bootstrap nodes found. Am I the only one? :(");
        }
        for maddr in bootstrap_nodes {
            self.dial(maddr)
        }
    }

    pub fn bootstrap(&mut self) {
        self.particle.bootstrap()
    }

    pub(super) fn dial(&mut self, maddr: Multiaddr) {
        self.events
            .push_back(libp2p::swarm::NetworkBehaviourAction::DialAddress { address: maddr })
    }

    event_polling!(custom_poll, events, SwarmEventType);
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ServerBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<PingEvent> for ServerBehaviour {
    fn inject_event(&mut self, _: PingEvent) {}
}
