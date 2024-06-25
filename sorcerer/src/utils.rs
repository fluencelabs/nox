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

use particle_args::JError;
use particle_execution::ParticleParams;

pub(crate) fn parse_spell_id_from(particle: &ParticleParams) -> Result<String, JError> {
    ParticleParams::get_spell_id(&particle.id).ok_or(JError::new(format!(
        "Invalid particle id: expected 'spell_{{SPELL_ID}}_{{COUNTER}}', got {}",
        particle.id
    )))
}
