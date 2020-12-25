mod args;
mod args_error;
mod closure;

pub use args::Args;
pub use args_error::ArgsError;
pub use closure::{
    closure, closure_args, closure_opt, Closure, ClosureDescriptor, FCEServiceClosure,
};

pub use aquamarine_vm::ParticleParams;
