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

use crate::error::AquamarineApiError;
use crate::observation::Observation;
use crate::ParticleEffects;

use fluence_libp2p::types::OneshotOutlet;

use std::ops::Deref;

pub type EffectsChannel = OneshotOutlet<Result<ParticleEffects, AquamarineApiError>>;

#[derive(Debug)]
/// A particle scheduled for execution.
/// Execution will produce ParticleEffects, which are to be sent to `out`
pub struct AwaitedParticle {
    pub particle: Observation,
    pub out: EffectsChannel,
}

impl From<AwaitedParticle> for (Observation, EffectsChannel) {
    fn from(item: AwaitedParticle) -> (Observation, EffectsChannel) {
        (item.particle, item.out)
    }
}

impl AsRef<Observation> for AwaitedParticle {
    fn as_ref(&self) -> &Observation {
        &self.particle
    }
}

impl Deref for AwaitedParticle {
    type Target = Observation;

    fn deref(&self) -> &Self::Target {
        &self.particle
    }
}

#[derive(Debug)]
/// Effects produced by particle execution along with destination that waits for those effects
///
/// Kind of like a completed promise
pub struct AwaitedEffects {
    /// Description of effects (e.g next_peer_pks of InterpreterOutcome) produced by particle execution
    /// or an error
    pub effects: Result<ParticleEffects, AquamarineApiError>,
    /// Destination that waits to receive ParticleEffects produced by particle execution
    pub out: EffectsChannel,
}

impl AwaitedEffects {
    pub fn ok(effects: ParticleEffects, out: EffectsChannel) -> Self {
        Self {
            effects: Ok(effects),
            out,
        }
    }

    pub fn expired(particle: AwaitedParticle) -> Self {
        let out = particle.out;
        let particle_id = particle.particle.particle().id;
        Self {
            out,
            effects: Err(AquamarineApiError::ParticleExpired { particle_id }),
        }
    }

    pub fn err(err: AquamarineApiError, out: EffectsChannel) -> Self {
        Self {
            effects: Err(err),
            out,
        }
    }
}
