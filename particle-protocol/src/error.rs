/*
 * Copyright 2023 Fluence Labs Limited
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
    #[error("Failed to verify particle {particle_id} signature: {err}")]
    SignatureVerificationFailed {
        #[source]
        err: VerificationError,
        particle_id: String,
    },
    #[error("Failed to decode public key from init_peer_id of particle {particle_id}: {err}")]
    DecodingError {
        #[source]
        err: fluence_keypair::error::DecodingError,
        particle_id: String,
    },
}
