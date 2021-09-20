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

use avm_server::CallResults;
use particle_protocol::Particle;
use std::ops::Deref;

/// Observed Particle.
///
/// On `First` observation it's just a particle, but on the `Next` observations
/// it might have `CallResults` attached to id.
#[derive(Debug)]
pub enum Observation {
    First(Particle),
    Next {
        particle: Particle,
        results: CallResults,
    },
}

impl Observation {
    pub fn particle(self) -> Particle {
        match self {
            Observation::First(p) => p,
            Observation::Next { particle, .. } => particle,
        }
    }
}

impl Deref for Observation {
    type Target = Particle;

    fn deref(&self) -> &Self::Target {
        match self {
            Observation::First(p) => p,
            Observation::Next { particle, .. } => particle,
        }
    }
}

impl From<Observation> for (Particle, CallResults) {
    fn from(this: Observation) -> (Particle, CallResults) {
        match this {
            Observation::First(p) => (p, <_>::default()),
            Observation::Next { particle, results } => (particle, results),
        }
    }
}
