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

#[allow(dead_code)]
// Enables logging, filtering out unnecessary details
pub fn enable_logs() {
    use log::LevelFilter::*;

    std::env::set_var("WASM_LOG", "info");

    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Off)
        .filter(Some("memory_delta"), Trace)
        // .filter(Some("script_storage"), Trace)
        // .filter(Some("aquamarine"), Trace)
        // .filter(Some("network"), Trace)
        // .filter(Some("network_api"), Trace)
        // .filter(Some("aquamarine::actor"), Debug)
        // .filter(Some("particle_node::bootstrapper"), Info)
        // .filter(Some("yamux::connection::stream"), Info)
        // .filter(Some("tokio_threadpool"), Info)
        // .filter(Some("tokio_reactor"), Info)
        // .filter(Some("mio"), Info)
        // .filter(Some("tokio_io"), Info)
        // .filter(Some("soketto"), Info)
        // .filter(Some("yamux"), Info)
        // .filter(Some("multistream_select"), Info)
        // .filter(Some("libp2p_swarm"), Info)
        // .filter(Some("libp2p_secio"), Info)
        // .filter(Some("libp2p_websocket::framed"), Info)
        // .filter(Some("libp2p_ping"), Info)
        // .filter(Some("libp2p_core::upgrade::apply"), Info)
        // .filter(Some("libp2p_kad::kbucket"), Info)
        // .filter(Some("libp2p_kad"), Info)
        // .filter(Some("libp2p_kad::query"), Info)
        // .filter(Some("libp2p_kad::iterlog"), Info)
        // .filter(Some("libp2p_plaintext"), Info)
        // .filter(Some("libp2p_identify::protocol"), Info)
        // .filter(Some("cranelift_codegen"), Info)
        // .filter(Some("wasmer_wasi"), Info)
        // .filter(Some("wasmer_interface_types_fl"), Info)
        // .filter(Some("async_std"), Info)
        // .filter(Some("async_io"), Info)
        // .filter(Some("polling"), Info)
        // .filter(Some("cranelift_codegen"), Info)
        // .filter(Some("walrus"), Info)
        .try_init()
        .ok();
}
