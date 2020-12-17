/*
 * Copyright 2020 Fluence Labs Limited
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

#![recursion_limit = "512"]
#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod server;
mod behaviour {
    mod bootstrapper;
    mod identify;
    mod server_behaviour;

    pub use server_behaviour::ServerBehaviour;
}

pub mod config {
    mod app_services;
    mod args;
    mod behaviour_config;
    mod defaults;
    mod fluence_config;
    mod keys;

    pub mod certificates;

    pub use app_services::AppServicesConfig;
    pub use args::create_args;
    pub use behaviour_config::BehaviourConfig;
    pub use defaults::default_air_interpreter_path;
    pub use fluence_config::load_config;
    pub use fluence_config::FluenceConfig;
    pub use fluence_config::ServerConfig;
}

mod bootstrapper {
    mod behaviour;
    mod event;

    pub use behaviour::BootstrapConfig;
    pub(crate) use behaviour::Bootstrapper;
    pub(crate) use event::BootstrapperEvent;
}

pub use behaviour::ServerBehaviour;
pub use bootstrapper::BootstrapConfig;
pub use server::Server;
