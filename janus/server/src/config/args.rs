/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
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
