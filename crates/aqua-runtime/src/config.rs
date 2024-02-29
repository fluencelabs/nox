use libp2p_identity::PeerId;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct VmPoolConfig {
    /// Number of VMs to create
    pub pool_size: usize,
    /// Timeout of a particle execution
    pub execution_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub current_peer_id: PeerId,
    /// Path to AIR interpreter .wasm file (aquamarine.wasm)
    pub air_interpreter: PathBuf,
    /// Maximum heap size in bytes available for the interpreter.
    pub max_heap_size: Option<u64>,
}

impl VmPoolConfig {
    pub fn new(pool_size: usize, execution_timeout: Duration) -> Self {
        Self {
            pool_size,
            execution_timeout,
        }
    }
}

impl VmConfig {
    pub fn new(
        current_peer_id: PeerId,
        air_interpreter: PathBuf,
        max_heap_size: Option<u64>,
    ) -> Self {
        Self {
            current_peer_id,
            air_interpreter,
            max_heap_size,
        }
    }
}
