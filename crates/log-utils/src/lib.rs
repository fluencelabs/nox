/*
 * Copyright 2021 Fluence Labs Limited
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

use tracing_subscriber::filter::LevelFilter;

#[allow(dead_code)]
// Enables logging, filtering out unnecessary details
pub fn enable_logs() {
    std::env::set_var("WASM_LOG", "info");

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy()
                .add_directive("builtins_deployer=trace".parse().unwrap())
                .add_directive("script_storage=trace".parse().unwrap())
                .add_directive("run-console=trace".parse().unwrap())
                .add_directive("sorcerer=trace".parse().unwrap())
                .add_directive("key_manager=trace".parse().unwrap())
                .add_directive("spell_event_bus=trace".parse().unwrap())
                .add_directive("aquamarine=trace".parse().unwrap())
                .add_directive("network=trace".parse().unwrap())
                .add_directive("network_api=trace".parse().unwrap())
                .add_directive("aquamarine::actor=debug".parse().unwrap())
                .add_directive("particle_node::bootstrapper=info".parse().unwrap())
                .add_directive("yamux::connection::stream=info".parse().unwrap())
                .add_directive("tokio_threadpool=info".parse().unwrap())
                .add_directive("tokio_reactor=info".parse().unwrap())
                .add_directive("mio=info".parse().unwrap())
                .add_directive("tokio_io=info".parse().unwrap())
                .add_directive("soketto=info".parse().unwrap())
                .add_directive("yamix=info".parse().unwrap())
                .add_directive("multistream_select=info".parse().unwrap())
                .add_directive("libp2p_swarm=info".parse().unwrap())
                .add_directive("libp2p_secio=info".parse().unwrap())
                .add_directive("libp2p_websocket::framed=info".parse().unwrap())
                .add_directive("libp2p_ping=info".parse().unwrap())
                .add_directive("libp2p_core::upgrade::apply=info".parse().unwrap())
                .add_directive("libp2p_kad::kbucket=info".parse().unwrap())
                .add_directive("libp2p_kad=info".parse().unwrap())
                .add_directive("libp2p_kad::query=info".parse().unwrap())
                .add_directive("libp2p_kad::iterlog=info".parse().unwrap())
                .add_directive("libp2p_plaintext=info".parse().unwrap())
                .add_directive("libp2p_identify::protocol=info".parse().unwrap())
                .add_directive("cranelift_codegen=off".parse().unwrap())
                .add_directive("cranelift_codegen::context=off".parse().unwrap())
                .add_directive("wasmer_wasi=info".parse().unwrap())
                .add_directive("wasmer_interface_types_fl=info".parse().unwrap())
                .add_directive("polling=info".parse().unwrap())
                .add_directive("walrus=info".parse().unwrap()),
        )
        .try_init()
        .ok();
}

pub fn enable_console() {
    console_subscriber::init();
}
