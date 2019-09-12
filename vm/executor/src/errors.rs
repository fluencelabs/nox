//use wasmer_runtime::error::*;

pub enum InstantiateError {
    Serialization,
    Deserialization,
    BadMemorySize,
}

pub enum ExecutionError {}
