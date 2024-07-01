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

use crate::behaviour::FluenceClientBehaviour;
use libp2p::PeerId;
use particle_protocol::Particle;

pub trait ParticleApi {
    fn send(&mut self, peer_id: PeerId, particle: Particle);
}

impl ParticleApi for FluenceClientBehaviour {
    fn send(&mut self, peer_id: PeerId, particle: Particle) {
        self.call(peer_id, particle)
    }
}
