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

#![feature(try_trait_v2)]
#![feature(exhaustive_patterns)]

pub use function_outcome::FunctionOutcome;
pub use particle_function::{
    Output as ParticleFunctionOutput, ParticleFunction, ParticleFunctionMut,
    ParticleFunctionStatic, ServiceFunction, ServiceFunctionImmut, ServiceFunctionMut,
};
pub use particle_params::ParticleParams;
pub use particle_vault::{ParticleVault, VaultError, VIRTUAL_PARTICLE_VAULT_PREFIX};

mod function_outcome;
mod particle_function;
mod particle_params;
mod particle_vault;
