mod args;
mod args_error;
mod closure;

pub use args::Args;
pub use args_error::ArgsError;
pub use closure::{
    closure, closure_args, closure_opt, closure_params, Closure, ClosureDescriptor, ParticleClosure,
};

pub use aquamarine_vm::ParticleParameters;
