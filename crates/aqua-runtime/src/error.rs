use avm_server::RunnerError;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ExecutionError {
    InvalidResultField {
        field: &'static str,
        error: FieldError,
    },
    AquamarineError(RunnerError),
}

impl Error for ExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            ExecutionError::InvalidResultField { error, .. } => Some(error),
            ExecutionError::AquamarineError(err) => Some(err),
        }
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::InvalidResultField { field, error } => {
                write!(f, "Execution error: invalid result field {field}: {error}")
            }
            ExecutionError::AquamarineError(err) => {
                write!(f, "Execution error: aquamarine error: {err}")
            }
        }
    }
}

#[derive(Debug)]
pub enum FieldError {
    InvalidPeerId { peer_id: String, err: String },
}

impl Error for FieldError {}
impl Display for FieldError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldError::InvalidPeerId { peer_id, err } => {
                write!(f, "invalid PeerId '{peer_id}': {err}")
            }
        }
    }
}
