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

use fluence_app_service::ParticleParameters;
use fluence_libp2p::PeerId;
use particle_protocol::Particle;
use types::peer_scope::PeerScope;

/// Lightweight, static version of the [[Particle]] structure
/// It exists to avoid cloning [[Particle::data]] when possible
#[derive(Debug, Clone)]
pub struct ParticleParams {
    pub id: String,
    pub init_peer_id: PeerId,
    pub peer_scope: PeerScope,
    /// Unix timestamp in milliseconds
    pub timestamp: u64,
    /// TTL in milliseconds
    pub ttl: u32,
    pub script: String,
    pub signature: Vec<u8>,
    // Particle token, `signature` signed with the peer's private key
    pub token: String,
}

impl ParticleParams {
    pub fn clone_from(particle: &Particle, peer_scope: PeerScope, token: String) -> Self {
        let Particle {
            id,
            init_peer_id,
            timestamp,
            ttl,
            script,
            signature,
            ..
        } = particle;

        Self {
            id: id.clone(),
            init_peer_id: *init_peer_id,
            peer_scope,
            timestamp: *timestamp,
            ttl: *ttl,
            script: script.clone(),
            signature: signature.clone(),
            token,
        }
    }

    pub fn is_spell_particle(particle_id: &str) -> bool {
        particle_id.starts_with("spell")
    }

    pub fn get_spell_id(particle_id: &str) -> Option<String> {
        if ParticleParams::is_spell_particle(particle_id) {
            particle_id.split('_').nth(1).map(|s| s.to_string())
        } else {
            None
        }
    }

    pub fn to_particle_parameters(self) -> ParticleParameters {
        ParticleParameters {
            id: self.id,
            init_peer_id: self.init_peer_id.to_string(),
            timestamp: self.timestamp,
            ttl: self.ttl,
            script: self.script,
            signature: self.signature,
            token: self.token,
        }
    }
}
