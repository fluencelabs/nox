/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
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

pub mod config {
    mod args;
    mod janus_config;
    mod keys;

    pub mod certificates;

    pub use self::args::create_args;
    pub use self::janus_config::load_config;
    pub use self::janus_config::JanusConfig;
    pub use self::janus_config::ServerConfig;
}

mod server;
mod behaviour {
    mod bootstrapper;
    mod identify;
    mod server_behaviour;

    pub use server_behaviour::ServerBehaviour;
}

mod function {
    mod builtin_service;
    mod dht_names;
    mod execution;
    mod peers;
    mod router;
    mod router_behaviour;
    mod services;
    mod waiting_queues;

    pub use router::FunctionRouter;
    pub use router::SwarmEventType;
}

mod bootstrapper {
    mod behaviour;
    mod event;

    pub use behaviour::Bootstrapper;
    pub use event::BootstrapperEvent;
}

pub mod kademlia {
    #![allow(clippy::mutable_key_type)]

    pub mod memory_store;
    pub mod record;

    pub(crate) use memory_store::MemoryStore;
    pub(crate) use record::expand_record_set;
}

pub(crate) use bootstrapper::Bootstrapper;
pub(crate) use bootstrapper::BootstrapperEvent;
pub(crate) use function::FunctionRouter;

pub use server::Server;

// #[cfg(any(tests, test))]
pub use behaviour::ServerBehaviour;
