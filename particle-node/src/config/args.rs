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
        // networking
        Arg::with_name(TCP_PORT)
            .takes_value(true)
            .short("t")
            .long("tcp-port")
            .help("tcp port [default: 7777]"),
        Arg::with_name(WEBSOCKET_PORT)
            .takes_value(true)
            .short("w")
            .long("ws-port")
            .help("websocket port [default: 9999]"),
        Arg::with_name(BOOTSTRAP_NODE)
            .value_name("MULTIADDR")
            .takes_value(true)
            .short("b")
            .long("bootstraps")
            .multiple(true)
            .empty_values(false)
            .help("bootstrap nodes of the Fluence network"),
        Arg::with_name(LOCAL)
            .short("l")
            .long("local")
            .takes_value(false)
            .conflicts_with(BOOTSTRAP_NODE)
            .help("if passed, bootstrap nodes aren't used"),
        Arg::with_name(EXTERNAL_ADDR)
            .takes_value(true)
            .short("x")
            .long("external-ip")
            .help("node external IP address to advertise to other peers"),
        Arg::with_name(EXTERNAL_MULTIADDRS)
            .takes_value(true)
            .multiple(true)
            .short("z")
            .long("external-maddrs")
            .help("node external multiaddresses to advertize to other peers"),
        // keypair
        Arg::with_name(ROOT_KEY_PAIR_VALUE)
            .takes_value(true)
            .short("k")
            .long("keypair-value")
            .help("keypair in base58 (conflicts with --keypair-path)")
            .conflicts_with(ROOT_KEY_PAIR_PATH),
        Arg::with_name(ROOT_KEY_PAIR_PATH)
            .takes_value(true)
            .short("p")
            .long("keypair-path")
            .help("keypair path (conflicts with --keypair-value)")
            .conflicts_with(ROOT_KEY_PAIR_VALUE),
        Arg::with_name(ROOT_KEY_PAIR_FORMAT)
            .takes_value(true)
            .short("f")
            .long("keypair-format")
            .help("keypair format")
            .possible_values(&["ed25519", "secp256k1", "rsa"]),
        Arg::with_name(ROOT_KEY_PAIR_GENERATE)
            .takes_value(true)
            .short("g")
            .long("gen-keypair")
            .help("generate keypair on absence"),
        // node configuration
        Arg::with_name(CONFIG_FILE)
            .takes_value(true)
            .short("c")
            .long("config")
            .help("TOML configuration file"),
        Arg::with_name(CERTIFICATE_DIR)
            .takes_value(true)
            .short("d")
            .long("cert-dir")
            .help("path to certificate dir"),
        Arg::with_name(MANAGEMENT_PEER_ID)
            .takes_value(true)
            .long("management-key")
            .short("m")
            .multiple(false)
            .help("PeerId of the node's administrator")
            .long_help("Peer with that peerID will have administrator privileges (e.g. add and remove aliases)"),
        // services
        Arg::with_name(SERVICE_ENVS)
            .value_name("NAME=VALUE")
            .takes_value(true)
            .short("e")
            .long("service-envs")
            .multiple(true)
            .empty_values(false)
            .help("envs to pass to core modules"),
        Arg::with_name(BLUEPRINT_DIR)
            .takes_value(true)
            .short("u")
            .long("blueprint-dir")
            .help("path to directory containing blueprints and wasm modules"),
        Arg::with_name(SERVICES_WORKDIR)
            .takes_value(true)
            .short("r")
            .long("services-workdir")
            .help("path to a directory where all services will store their data"),
    ]
}
