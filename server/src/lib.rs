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

/// Takes option as an argument, unwraps if `Some`, exit function with `Ok(default)` otherwise
/// Ought to make it easier to short-circuit in functions returning `Result<Option<_>>`
macro_rules! ok_get {
    ($opt:expr) => {{
        let r = { $opt };
        match r {
            Some(r) => r,
            None => return Ok(Default::default()),
        }
    }};
}

pub mod config {
    mod args;
    mod fluence_config;
    mod keys;

    pub mod certificates;

    pub use self::args::create_args;
    pub use self::fluence_config::load_config;
    pub use self::fluence_config::FluenceConfig;
    pub use self::fluence_config::ServerConfig;
}

mod server;
mod behaviour {
    mod bootstrapper;
    mod identify;
    mod server_behaviour;

    pub use server_behaviour::ServerBehaviour;
}

mod function {
    mod address_signature;
    mod builtin_service;
    mod config;
    mod dht_names;
    mod errors;
    mod execution;
    mod peers;
    mod provider_record;
    mod router;
    mod router_behaviour;
    mod services;
    mod wait_peer;
    mod waiting_queues;

    pub(crate) use self::config::RouterConfig;
    pub(self) use errors::{CallError, CallErrorKind, ErrorData};
    pub(crate) use router::FunctionRouter;
    pub(self) use router::SwarmEventType;
    pub(self) use services::ResolvedFunction;
}

mod bootstrapper {
    mod behaviour;
    mod event;

    pub use behaviour::BootstrapConfig;
    pub(crate) use behaviour::Bootstrapper;
    pub(crate) use event::BootstrapperEvent;
}

pub mod kademlia {
    #![allow(clippy::mutable_key_type)]

    pub mod memory_store;
    pub mod record;

    pub(crate) use memory_store::MemoryStore;
    pub(crate) use record::try_to_multirecord;
}

pub mod faas {
    mod behaviour;
    pub(crate) use behaviour::{FaaSBehaviour, FaaSCall};
}

pub(crate) use bootstrapper::Bootstrapper;
pub(crate) use bootstrapper::BootstrapperEvent;
pub(crate) use function::FunctionRouter;

pub use behaviour::ServerBehaviour;
pub use bootstrapper::BootstrapConfig;
pub use server::Server;
