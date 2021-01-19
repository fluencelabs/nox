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

use crate::StepperEffects;
use fluence_libp2p::types::OneshotOutlet;
use particle_protocol::Particle;
use std::ops::Deref;

#[derive(Debug)]
/// A particle scheduled for execution.
/// Execution will produce StepperEffects, which are to be sent to `out`
pub struct AwaitedParticle {
    pub particle: Particle,
    pub out: OneshotOutlet<StepperEffects>,
}

impl Into<(Particle, OneshotOutlet<StepperEffects>)> for AwaitedParticle {
    fn into(self) -> (Particle, OneshotOutlet<StepperEffects>) {
        (self.particle, self.out)
    }
}

impl AsRef<Particle> for AwaitedParticle {
    fn as_ref(&self) -> &Particle {
        &self.particle
    }
}

impl Deref for AwaitedParticle {
    type Target = Particle;

    fn deref(&self) -> &Self::Target {
        &self.particle
    }
}

#[derive(Debug)]
/// Effects produced by particle execution along with destination that waits for those effects
///
/// Kind of like a completed promise
pub struct AwaitedEffects {
    /// Description of effects (e.g next_peer_pks of StepperOutcome) produced by particle execution
    pub effects: StepperEffects,
    /// Destination that waits to receive StepperEffects produced by particle execution
    pub out: OneshotOutlet<StepperEffects>,
}
