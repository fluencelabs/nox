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

use config_utils::to_abs_path;
use rand::Rng;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub type Result<T> = eyre::Result<T>;

pub fn uuid() -> String {
    Uuid::new_v4().to_string()
}

#[allow(dead_code)]
// Enables logging, filtering out unnecessary details
pub fn enable_logs() {
    use log::LevelFilter::*;

    std::env::set_var("WASM_LOG", "info");

    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Info)
        .filter(Some("aquamarine"), Trace)
        .filter(Some("aquamarine::actor"), Debug)
        .filter(Some("particle_node::bootstrapper"), Info)
        .filter(Some("yamux::connection::stream"), Info)
        .filter(Some("tokio_threadpool"), Info)
        .filter(Some("tokio_reactor"), Info)
        .filter(Some("mio"), Info)
        .filter(Some("tokio_io"), Info)
        .filter(Some("soketto"), Info)
        .filter(Some("yamux"), Info)
        .filter(Some("multistream_select"), Info)
        .filter(Some("libp2p_swarm"), Info)
        .filter(Some("libp2p_secio"), Info)
        .filter(Some("libp2p_websocket::framed"), Info)
        .filter(Some("libp2p_ping"), Info)
        .filter(Some("libp2p_core::upgrade::apply"), Info)
        .filter(Some("libp2p_kad::kbucket"), Info)
        .filter(Some("libp2p_kad"), Info)
        .filter(Some("libp2p_kad::query"), Info)
        .filter(Some("libp2p_kad::iterlog"), Info)
        .filter(Some("libp2p_plaintext"), Info)
        .filter(Some("libp2p_identify::protocol"), Info)
        .filter(Some("cranelift_codegen"), Info)
        .filter(Some("wasmer_wasi"), Info)
        .filter(Some("wasmer_interface_types_fl"), Info)
        .filter(Some("async_std"), Info)
        .filter(Some("async_io"), Info)
        .filter(Some("polling"), Info)
        .filter(Some("cranelift_codegen"), Info)
        .filter(Some("walrus"), Info)
        .try_init()
        .ok();
}

pub fn make_tmp_dir() -> PathBuf {
    use rand::distributions::Alphanumeric;

    let mut tmp = std::env::temp_dir();
    tmp.push("fluence_test/");
    let dir: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(16)
        .collect();
    tmp.push(dir);

    std::fs::create_dir_all(&tmp).expect("create tmp dir");

    tmp
}

pub fn remove_dir(dir: &Path) {
    std::fs::remove_dir_all(&dir).unwrap_or_else(|_| panic!("remove dir {:?}", dir))
}

pub fn put_aquamarine(tmp: PathBuf) -> PathBuf {
    use air_interpreter_wasm::{INTERPRETER_WASM, VERSION};

    std::fs::create_dir_all(&tmp).expect("create tmp dir");

    let file = to_abs_path(tmp.join(format!("aquamarine_{}.wasm", VERSION)));
    std::fs::write(&file, INTERPRETER_WASM)
        .unwrap_or_else(|_| panic!("fs::write aquamarine.wasm to {:?}", file));

    file
}

pub fn load_module(path: &str, module_name: &str) -> Vec<u8> {
    let module = to_abs_path(PathBuf::from(path).join(format!("{}.wasm", module_name)));
    std::fs::read(&module).unwrap_or_else(|_| panic!("fs::read from {:?}", module))
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis()
}
