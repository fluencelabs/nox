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

use fluence_spell_dtos::value::SpellValueT;
use serde::de::DeserializeOwned;

use particle_args::JError;
use particle_execution::{FunctionOutcome, ParticleParams};

pub(crate) fn parse_spell_id_from(particle: &ParticleParams) -> Result<String, JError> {
    ParticleParams::get_spell_id(&particle.id).ok_or(JError::new(format!(
        "Invalid particle id: expected 'spell_{{SPELL_ID}}_{{COUNTER}}', got {}",
        particle.id
    )))
}
