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

use log::Level;
use tracing_subscriber::filter::{Directive, LevelFilter};

fn default_directives() -> Vec<Directive> {
    let namespaces = vec![
        "script_storage=trace",
        "run-console=trace",
        "sorcerer=trace",
        "key_manager=trace",
        "spell_event_bus=trace",
        "aquamarine=trace",
        "network=trace",
        "network_api=trace",
        "aquamarine::actor=debug",
        "nox::bootstrapper=info",
        "yamux::connection::stream=info",
        "tokio_threadpool=info",
        "tokio_reactor=info",
        "mio=info",
        "tokio_io=info",
        "soketto=info",
        "yamix=info",
        "multistream_select=info",
        "libp2p_swarm=info",
        "libp2p_secio=info",
        "libp2p_websocket::framed=info",
        "libp2p_ping=info",
        "libp2p_core::upgrade::apply=info",
        "libp2p_kad::kbucket=info",
        "libp2p_kad=info",
        "libp2p_kad::query=info",
        "libp2p_kad::iterlog=info",
        "libp2p_plaintext=info",
        "libp2p_identify::protocol=info",
        "cranelift_codegen=off",
        "cranelift_codegen::context=off",
        "wasmer_wasi=info",
        "wasmer_interface_types_fl=info",
        "polling=info",
        "walrus=info",
        "regalloc2=info",
        "cranelift_wasm=info",
        "wasmtime_cranelift=info",
        "tokio=info",
        "libp2p_noise=info",
        "yamux=info",
        "wasmtime_jit=info",
        "wasi_common=info",
        "particle_reap=info",
        "marine_core::module::marine_module=info",
        "runtime::resource=info",
    ];

    namespaces
        .into_iter()
        .map(|ns| {
            ns.parse()
                .expect(format!("cannot parse {ns} to Directive").as_str())
        })
        .collect()
}

#[allow(dead_code)]
// Enables logging, filtering out unnecessary details
pub fn enable_logs() {
    enable_logs_for(LogSpec::default())
}

pub fn enable_console() {
    console_subscriber::init();
}

pub struct LogSpec {
    directives: Vec<Directive>,
    wasm_log: Level,
}

impl Default for LogSpec {
    fn default() -> Self {
        Self::new(vec![])
            .with_defaults()
            .with_wasm_level(Level::Info)
    }
}

impl LogSpec {
    pub fn new(directives: Vec<Directive>) -> Self {
        Self {
            directives,
            wasm_log: Level::Info,
        }
    }

    pub fn with_defaults(mut self) -> Self {
        self.directives = default_directives()
            .into_iter()
            .chain(self.directives)
            .collect();

        self
    }

    pub fn with_wasm_level(mut self, level: Level) -> Self {
        self.wasm_log = level;

        self
    }
}

pub fn enable_logs_for(spec: LogSpec) {
    std::env::set_var("WASM_LOG", spec.wasm_log.to_string().to_lowercase());

    let mut filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::TRACE.into())
        .from_env_lossy();

    for d in spec.directives {
        filter = filter.add_directive(d);
    }

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .ok();
}
