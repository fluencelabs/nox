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

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum AquamarineApiError {
    ParticleExpired { particle_id: String },
    OneshotCancelled { particle_id: String },
    AquamarineDied { particle_id: String },
}

impl AquamarineApiError {
    pub fn into_particle_id(self) -> String {
        match self {
            AquamarineApiError::ParticleExpired { particle_id } => particle_id,
            AquamarineApiError::OneshotCancelled { particle_id } => particle_id,
            AquamarineApiError::AquamarineDied { particle_id } => particle_id,
        }
    }
}

impl Error for AquamarineApiError {}

impl Display for AquamarineApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AquamarineApiError::ParticleExpired { particle_id } => {
                write!(
                    f,
                    "AquamarineApiError::ParticleExpired: particle_id = {}",
                    particle_id
                )
            }
            AquamarineApiError::OneshotCancelled { particle_id } => {
                write!(
                    f,
                    "AquamarineApiError::OneshotCancelled: particle_id = {}. \
                     Aquamarine dropped particle processing before sending effects back. \
                     This is unexpected and shouldn't happen.",
                    particle_id
                )
            }
            AquamarineApiError::AquamarineDied { particle_id } => {
                write!(
                    f,
                    "AquamarineApiError::AquamarineDied: particle_id = {}. \
                     Aquamarine couldn't be reached from the NetworkApi. \
                     This is unexpected and shouldn't happen.",
                    particle_id
                )
            }
        }
    }
}
