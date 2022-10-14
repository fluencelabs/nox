#![feature(try_trait_v2)]
#![feature(exhaustive_patterns)]

pub use function_outcome::FunctionOutcome;
pub use particle_function::{
    Output as ParticleFunctionOutput, ParticleFunction, ParticleFunctionMut,
    ParticleFunctionStatic, ServiceFunction,
};
pub use particle_params::ParticleParams;
pub use particle_vault::{ParticleVault, VaultError};

mod function_outcome;
mod particle_function;
mod particle_params;
mod particle_vault;
