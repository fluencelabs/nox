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

use crate::config::janus_config::*;
use clap::Arg;

pub fn create_args<'a, 'b>() -> Vec<Arg<'a, 'b>> {
    vec![
        Arg::with_name(TCP_PORT)
            .takes_value(true)
            .short("t")
            .default_value("7777")
            .help("tcp port"),
        Arg::with_name(WEBSOCKET_PORT)
            .takes_value(true)
            .short("w")
            .default_value("9999")
            .help("websocket port"),
        Arg::with_name(ROOT_KEY_PAIR_PATH)
            .takes_value(true)
            .short("s")
            .conflicts_with(ROOT_KEY_PAIR)
            .help("path to ed25519 key pair file"),
        Arg::with_name(ROOT_KEY_PAIR)
            .takes_value(true)
            .short("k")
            .conflicts_with(ROOT_KEY_PAIR_PATH)
            .help("ed25519 key pair in base58"),
        Arg::with_name(CONFIG_FILE)
            .takes_value(true)
            .short("c")
            .help("TOML configuration file"),
        Arg::with_name(CERTIFICATE_DIR)
            .takes_value(true)
            .short("d")
            .help("path to certificate dir"),
        Arg::with_name(BOOTSTRAP_NODE)
            .takes_value(true)
            .short("b")
            .multiple(true)
            .help("bootstrap nodes of the Fluence network"),
        Arg::with_name(EXTERNAL_ADDR)
            .takes_value(true)
            .short("e")
            .help("external network address to publish as discoverable"),
    ]
}
