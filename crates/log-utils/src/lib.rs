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
        .filter_level(log::LevelFilter::Error)
        .filter(Some("aquamarine"), Error)
        .filter(Some("aquamarine::actor"), Error)
        .filter(Some("particle_node::bootstrapper"), Error)
        .filter(Some("yamux::connection::stream"), Error)
        .filter(Some("tokio_threadpool"), Error)
        .filter(Some("tokio_reactor"), Error)
        .filter(Some("mio"), Error)
        .filter(Some("tokio_io"), Error)
        .filter(Some("soketto"), Error)
        .filter(Some("yamux"), Error)
        .filter(Some("multistream_select"), Error)
        .filter(Some("libp2p_swarm"), Error)
        .filter(Some("libp2p_secio"), Error)
        .filter(Some("libp2p_websocket::framed"), Error)
        .filter(Some("libp2p_ping"), Error)
        .filter(Some("libp2p_core::upgrade::apply"), Error)
        .filter(Some("libp2p_kad::kbucket"), Error)
        .filter(Some("libp2p_kad"), Error)
        .filter(Some("libp2p_kad::query"), Error)
        .filter(Some("libp2p_kad::iterlog"), Error)
        .filter(Some("libp2p_plaintext"), Error)
        .filter(Some("libp2p_identify::protocol"), Error)
        .filter(Some("cranelift_codegen"), Error)
        .filter(Some("wasmer_wasi"), Error)
        .filter(Some("wasmer_interface_types_fl"), Error)
        .filter(Some("async_std"), Error)
        .filter(Some("async_io"), Error)
        .filter(Some("polling"), Error)
        .filter(Some("cranelift_codegen"), Error)
        .filter(Some("walrus"), Error)
        .try_init()
        .ok();
}
