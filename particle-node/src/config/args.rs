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

use server_config::config_keys::*;

use clap::Arg;

pub fn create_args<'a, 'b>() -> Vec<Arg<'a, 'b>> {
    vec![
        Arg::with_name(TCP_PORT)
            .takes_value(true)
            .short("t")
            .help("tcp port [default: 7777]"),
        Arg::with_name(WEBSOCKET_PORT)
            .takes_value(true)
            .short("w")
            .help("websocket port [default: 9999]"),
        Arg::with_name(ROOT_KEY_PAIR)
            .takes_value(true)
            .short("k")
            .help("path to ed25519 keypair or keypair in base58"),
        Arg::with_name(CONFIG_FILE)
            .takes_value(true)
            .short("c")
            .help("TOML configuration file"),
        Arg::with_name(CERTIFICATE_DIR)
            .takes_value(true)
            .short("d")
            .help("path to certificate dir"),
        Arg::with_name(BOOTSTRAP_NODE)
            .value_name("MULTIADDR")
            .takes_value(true)
            .short("b")
            .multiple(true)
            .help("bootstrap nodes of the Fluence network"),
        Arg::with_name(EXTERNAL_ADDR)
            .takes_value(true)
            .short("x")
            .help("external network address to publish as discoverable"),
        Arg::with_name(SERVICE_ENVS)
            .value_name("NAME=VALUE")
            .takes_value(true)
            .short("e")
            .multiple(true)
            .help("envs to pass to core modules"),
        Arg::with_name(BLUEPRINT_DIR)
            .takes_value(true)
            .long("blueprint-dir")
            .multiple(true)
            .help("path to directory containing blueprints and wasm modules"),
        Arg::with_name(SERVICES_WORKDIR)
            .takes_value(true)
            .long("services-workdir")
            .multiple(true)
            .help("path to a directory where all services will store their data"),
        Arg::with_name(MANAGEMENT_KEY)
            .takes_value(true)
            .long("management-key")
            .multiple(false)
            .help("a key (PeerId) that will be used to manage a node like adding aliases to services"),
    ]
}
