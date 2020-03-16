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

#![recursion_limit = "512"]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

use janus_client::client::Client;

use async_std::{io, task};
use env_logger;
use futures::prelude::*;
use futures::{select, stream::StreamExt};
use janus_client::TextCommand;
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use parity_multihash::{Hash, Multihash};
use serde_json;
use std::convert::TryInto;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let relay_addr: Multiaddr = std::env::args()
        .nth(1)
        .expect("multiaddr of relay peer should be provided by the first argument")
        .parse()
        .expect("provided wrong  Multiaddr");

    let relay_id: PeerId = std::env::args()
        .nth(2)
        .expect("peer id should be provided by the second argument")
        .parse()
        .expect("provided wrong PeerId");

    let client = Client::connect(relay_addr, relay_id);

    task::block_on(async move {
        let mut client = client.await?;
        print_example(&client.peer_id);

        let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

        loop {
            select!(
                from_stdin = stdin.select_next_some() => {
                    match from_stdin {
                        Ok(line) => {
                            let cmd: Result<TextCommand, _> = serde_json::from_str(&line);
                            if let Ok(cmd) = cmd {
                                client.send(cmd.try_into().unwrap());
                            } else {
                                println!("incorrect string provided");
                            }
                        }
                        Err(_) => panic!("Stdin closed"),
                    }
                },
                incoming = client.receive_one() => {
                    match incoming {
                        Some(msg) => println!("Received {}", msg),
                        None => break
                    }
                }
            )
        }

        Ok(())
    })
}

fn print_example(peer_id: &PeerId) {
    fn show(cmd: TextCommand) {
        println!("{}", serde_json::to_value(cmd).unwrap());
    }

    let relay_example = TextCommand::Relay {
        dst: peer_id.to_string(),
        message: "hello".to_string(),
    };

    let rnd = bs58::encode(Multihash::random(Hash::SHA2256)).into_string();
    let provide_example = TextCommand::Provide { key: rnd.clone() };
    let find_example = TextCommand::FindProviders { key: rnd };

    println!("possible messages:");
    show(relay_example);
    show(provide_example);
    show(find_example);
    println!("\n")
}
