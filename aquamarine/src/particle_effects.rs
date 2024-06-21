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

use std::time::Duration;

use libp2p::PeerId;

use avm_server::CallRequests;
use particle_protocol::ExtendedParticle;
use types::peer_scope::PeerScope;

#[derive(Clone, Debug)]
/// Effects produced by particle execution. Currently the only effect is that of sending particles.
pub struct ParticleEffects {
    /// New particle data
    pub new_data: Vec<u8>,
    /// Instruction to send particle to these peers
    pub next_peers: Vec<PeerId>,
    /// Instruction to execute host calls
    pub call_requests: CallRequests,
}

impl ParticleEffects {
    pub fn empty() -> Self {
        Self {
            new_data: vec![],
            next_peers: vec![],
            call_requests: <_>::default(),
        }
    }
}

#[derive(Clone, Debug)]
/// Performance stats about particle's interpretation
pub struct InterpretationStats {
    pub interpretation_time: Duration,
    pub memory_delta: usize,
    pub new_data_len: Option<usize>,
    pub success: bool,
}

impl InterpretationStats {
    pub fn failed() -> Self {
        Self {
            interpretation_time: Duration::default(),
            memory_delta: 0,
            new_data_len: None,
            success: false,
        }
    }
}

/// Routing part of the [[ParticleEffects].
/// Instruct to send particle to either virtual or remote peers.
#[derive(Clone, Debug)]
pub struct RawRoutingEffects {
    pub particle: ExtendedParticle,
    pub next_peers: Vec<PeerId>,
}

#[derive(Clone, Debug)]
pub struct RemoteRoutingEffects {
    pub particle: ExtendedParticle,
    pub next_peers: Vec<PeerId>,
}

#[derive(Clone, Debug)]
pub struct LocalRoutingEffects {
    pub particle: ExtendedParticle,
    pub next_peers: Vec<PeerScope>,
}
