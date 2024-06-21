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
use particle_protocol::ParticleError;
use particle_services::PeerScope;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SorcererError {
    #[error("Failed to sign particle for spell {spell_id} : {err}")]
    ParticleSigningFailed {
        #[source]
        err: ParticleError,
        spell_id: String,
    },
    #[error("Keypair for spell {spell_id}:{peer_scope:?} is missing")]
    ScopeKeypairMissing {
        spell_id: String,
        peer_scope: PeerScope,
    },
}
