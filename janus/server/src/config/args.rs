/*
 * Copyright 2019 Fluence Labs Limited
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

use crate::config::config::*;
use clap::Arg;

pub fn prepare_args<'a, 'b>() -> [Arg<'a, 'b>; 6] {
    [
        Arg::with_name(PEER_SERVICE_PORT)
            .takes_value(true)
            .short("o")
            .default_value("9999")
            .help("port that will be used by the peer service"),
        Arg::with_name(CLIENT_TYPE)
            .takes_value(true)
            .short("c")
            .default_value("websocket")
            .help("client's endpoint type: websocket, libp2p"),
        Arg::with_name(SECRET_KEY)
            .takes_value(true)
            .short("s")
            .help("ed25519 secret key in base64 format"),
        Arg::with_name(PEER_SECRET_KEY)
            .takes_value(true)
            .short("p")
            .help("ed25519 secret key for peer connection in base64 format"),
        Arg::with_name(NODE_SERVICE_PORT)
            .takes_value(true)
            .short("n")
            .default_value("7777")
            .help("port that will be used by the node service"),
        Arg::with_name(BOOTSTRAP_NODE)
            .takes_value(true)
            .short("b")
            .multiple(true)
            .help("bootstrap nodes of the Fluence network"),
    ]
}
