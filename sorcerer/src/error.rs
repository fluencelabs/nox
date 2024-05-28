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
