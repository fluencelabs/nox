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

use clap::Arg;

use server_config::config_keys::*;

pub fn create_args<'help>() -> Vec<Arg<'help>> {
    vec![
        // networking
        Arg::new(TCP_PORT)
            .display_order(1)
            .help_heading(Some("Networking"))
            .takes_value(true)
            .short('t')
            .long("tcp-port")
            .value_name("PORT")
            .default_value("7777")
            .about("tcp port"),
        Arg::new(WEBSOCKET_PORT)
            .display_order(2)
            .help_heading(Some("Networking"))
            .takes_value(true)
            .short('w')
            .long("ws-port")
            .value_name("PORT")
            .default_value("9999")
            .about("websocket port"),
        Arg::new(PROMETHEUS_PORT)
            .display_order(3)
            .help_heading(Some("Networking"))
            .takes_value(true)
            .short('s')
            .long("prometheus-port")
            .value_name("PORT")
            .default_value("18080")
            .about("prometheus metrics port"),
        Arg::new(EXTERNAL_ADDR)
            .display_order(4)
            .help_heading(Some("Networking"))
            .takes_value(true)
            .short('x')
            .long("external-ip")
            .value_name("IP")
            .about("node external IP address to advertise to other peers"),
        Arg::new(EXTERNAL_MULTIADDRS)
            .display_order(5)
            .help_heading(Some("Networking"))
            .takes_value(true)
            .multiple_values(true)
            .short('z')
            .long("external-maddrs")
            .value_name("MULTIADDR")
            .about("external multiaddresses to advertize"),
        Arg::new(ALLOW_PRIVATE_IPS)
            .display_order(6)
            .help_heading(Some("Networking"))
            .short('a')
            .long("allow-private-ips")
            .takes_value(false)
            .about("allow private IP addresses from other nodes"),
        Arg::new(BOOTSTRAP_NODE)
            .display_order(7)
            .help_heading(Some("Networking"))
            .value_name("MULTIADDR")
            .takes_value(true)
            .short('b')
            .long("bootstraps")
            .multiple_values(true)
            .about("bootstrap nodes of the Fluence network"),
        Arg::new(BOOTSTRAP_FREQ)
            .display_order(8)
            .help_heading(Some("Networking"))
            .value_name("N")
            .takes_value(true)
            .short('q')
            .long("bootstrap-freq")
            .about("bootstrap kademlia each time N bootstraps (re)connect"),
        Arg::new(LOCAL)
            .display_order(9)
            .help_heading(Some("Networking"))
            .short('l')
            .long("local")
            .takes_value(false)
            .conflicts_with(BOOTSTRAP_NODE)
            .about("if passed, bootstrap nodes aren't used"),
        // keypair
        Arg::new(ROOT_KEY_PAIR_VALUE)
            .display_order(10)
            .help_heading(Some("Node keypair"))
            .takes_value(true)
            .short('k')
            .long("keypair-value")
            .value_name("BYTES")
            .about("keypair in base58 (conflicts with --keypair-path)")
            .conflicts_with(ROOT_KEY_PAIR_PATH),
        Arg::new(ROOT_KEY_PAIR_PATH)
            .display_order(11)
            .help_heading(Some("Node keypair"))
            .takes_value(true)
            .short('p')
            .long("keypair-path")
            .about("keypair path (conflicts with --keypair-value)")
            .conflicts_with(ROOT_KEY_PAIR_VALUE),
        Arg::new(ROOT_KEY_PAIR_FORMAT)
            .display_order(12)
            .help_heading(Some("Node keypair"))
            .takes_value(true)
            .short('f')
            .long("keypair-format")
            .possible_values(&["ed25519", "secp256k1", "rsa"]),
        Arg::new(ROOT_KEY_PAIR_GENERATE)
            .display_order(13)
            .help_heading(Some("Node keypair"))
            .takes_value(true)
            .short('g')
            .long("gen-keypair")
            .about("generate keypair on absence"),
        // node configuration
        Arg::new(CONFIG_FILE)
            .display_order(14)
            .help_heading(Some("Node configuration"))
            .takes_value(true)
            .short('c')
            .long("config")
            .value_name("PATH")
            .about("TOML configuration file"),
        Arg::new(CERTIFICATE_DIR)
            .display_order(15)
            .help_heading(Some("Node configuration"))
            .takes_value(true)
            .short('d')
            .long("cert-dir")
            .value_name("PATH")
            .about("certificate dir"),
        Arg::new(MANAGEMENT_PEER_ID)
            .display_order(16)
            .help_heading(Some("Node configuration"))
            .takes_value(true)
            .long("management-key")
            .short('m')
            .value_name("PEER ID")
            .about("PeerId of the node's administrator"),
        // services
        Arg::new(SERVICE_ENVS)
            .display_order(17)
            .help_heading(Some("Services configuration"))
            .value_name("NAME=VALUE")
            .takes_value(true)
            .short('e')
            .long("service-envs")
            .multiple_values(true)
            .about("envs to pass to core modules"),
        Arg::new(BLUEPRINT_DIR)
            .display_order(18)
            .help_heading(Some("Services configuration"))
            .takes_value(true)
            .short('u')
            .long("blueprint-dir")
            .value_name("PATH")
            .about("directory containing blueprints and wasm modules"),
        Arg::new(SERVICES_WORKDIR)
            .display_order(19)
            .help_heading(Some("Services configuration"))
            .takes_value(true)
            .short('r')
            .long("services-workdir")
            .value_name("PATH")
            .about("directory where all services will store their data"),
    ]
}
