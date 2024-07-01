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
use thiserror::Error;

use fluence_keypair::error::{SigningError, VerificationError};

#[derive(Debug, Error)]
pub enum ParticleError {
    #[error("Cannot sign particle {particle_id} with keypair not from init_peer_id {init_peer_id}: given {given_peer_id}")]
    InvalidKeypair {
        particle_id: String,
        init_peer_id: String,
        given_peer_id: String,
    },
    #[error("Failed to sign particle {particle_id} signature: {err}")]
    SigningFailed {
        #[source]
        err: SigningError,
        particle_id: String,
    },
    #[error("Failed to verify particle {particle_id} by {peer_id} with signature: {err}")]
    SignatureVerificationFailed {
        #[source]
        err: VerificationError,
        particle_id: String,
        peer_id: String,
    },
    #[error("Failed to decode public key from init_peer_id of particle {particle_id}: {err}")]
    DecodingError {
        #[source]
        err: fluence_keypair::error::DecodingError,
        particle_id: String,
    },
}
