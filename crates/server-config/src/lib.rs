mod app_services;
mod behaviour_config;
mod bootstrap_config;
mod defaults;
mod fluence_config;
mod kademlia_config;
mod keys;

pub use defaults::default_air_interpreter_path;
pub use fluence_config::load_config;

pub use app_services::AppServicesConfig;
pub use behaviour_config::BehaviourConfig;
pub use bootstrap_config::BootstrapConfig;
pub use fluence_config::FluenceConfig;
pub use fluence_config::ServerConfig;
pub use kademlia_config::KademliaConfig;

pub mod config_keys {
    pub use crate::fluence_config::{
        BLUEPRINT_DIR, BOOTSTRAP_NODE, CERTIFICATE_DIR, CONFIG_FILE, EXTERNAL_ADDR, ROOT_KEY_PAIR,
        SERVICE_ENVS, TCP_PORT, WEBSOCKET_PORT,
    };
}
