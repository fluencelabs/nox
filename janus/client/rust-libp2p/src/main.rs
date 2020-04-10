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
use futures::prelude::*;
use futures::{select, stream::StreamExt};
use janus_client::Command;
use janus_server::node_service::function::{Address, FunctionCall};
use libp2p::PeerId;

use parity_multiaddr::Multiaddr;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let relay_addr: Multiaddr = std::env::args()
        .nth(1)
        .expect("multiaddr of relay peer should be provided by the first argument")
        .parse()
        .expect("provided wrong  Multiaddr");

    let bootstrap_id: PeerId = std::env::args()
        .nth(2)
        .expect("peer id should be provided by the second argument")
        .parse()
        .expect("provided wrong PeerId");

    let client = Client::connect(relay_addr, bootstrap_id.clone());

    task::block_on(async move {
        let (mut client, _client_task) = client.await?;
        print_example(&client.peer_id, &bootstrap_id);

        let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

        loop {
            select!(
                from_stdin = stdin.select_next_some() => {
                    match from_stdin {
                        Ok(line) => {
                            let cmd: Result<Command, _> = serde_json::from_str(&line);
                            if let Ok(cmd) = cmd {
                                client.send(cmd);
                            } else {
                                println!("incorrect string provided");
                            }
                        }
                        Err(_) => panic!("Stdin closed"),
                    }
                },
                incoming = client.receive_one() => {
                    match incoming {
                        Some(msg) => println!("Received {:?}", msg),
                        None => println!("client closed inlet")
                    }
                }
            )
        }

        // println!("Select stdout finished");
        // _client_task.await;
        // Ok(())
    })
}

fn print_example(peer_id: &PeerId, bootstrap: &PeerId) {
    fn show(cmd: Command) {
        println!("{}", serde_json::to_value(cmd).unwrap());
    }

    let call_example = Command::Call {
        call: FunctionCall {
            uuid: "UUID-1".to_string(),
            target: Some(Address::Peer {
                peer: bootstrap.clone(),
            }),
            reply_to: Some(Address::Peer {
                peer: peer_id.clone(),
            }),
            arguments: serde_json::Value::Null,
            name: Some("name!".to_string()),
        },
    };

    println!("possible messages:");
    show(call_example);
    println!("\n")
}
