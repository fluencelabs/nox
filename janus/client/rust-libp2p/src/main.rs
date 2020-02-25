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

use std::{error::Error, time::Duration};

use async_std::{io, task};
use env_logger;
use futures::prelude::*;
use futures::{select, stream::StreamExt};
use libp2p::{identity, PeerId};
use parity_multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use serde_json;

use janus_server::peer_service::libp2p::build_transport;

use crate::behaviour::ClientServiceBehaviour;
use crate::connect_protocol::events::InEvent;

mod behaviour;
mod connect_protocol;

// user input for relaying (just a json now)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct RelayUserInput {
    dst: String,
    message: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {}", local_peer_id);

    let relay_example = RelayUserInput {
        dst: local_peer_id.to_string(),
        message: "hello".to_string(),
    };
    let relay_example = serde_json::to_value(relay_example).unwrap();
    println!("example of a relay message: {}", relay_example.to_string());

    let mut swarm = {
        let transport = build_transport(local_key.clone(), Duration::from_secs(20));
        let behaviour = ClientServiceBehaviour::new(&local_peer_id, local_key.public());
        libp2p::Swarm::new(transport, behaviour, local_peer_id)
    };

    let relay_peer_addr: Multiaddr = std::env::args()
        .nth(1)
        .expect("multiaddr of relay peer should be provided by the first argument")
        .parse()
        .expect("provided wrong  Multiaddr");

    match libp2p::Swarm::dial_addr(&mut swarm, relay_peer_addr.clone()) {
        Ok(_) => println!("Dialed to {:?}", relay_peer_addr),
        Err(e) => {
            println!("Dial to {:?} failed with {:?}", relay_peer_addr, e);
        }
    }

    let relay_peer: PeerId = std::env::args()
        .nth(2)
        .expect("peer id should be provided by the second argument")
        .parse()
        .expect("provided wrong PeerId");

    task::block_on(async move {
        let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

        loop {
            select!(
            from_stdin = stdin.select_next_some() => {
                match from_stdin {
                    Ok(line) => {
                        let relay_user_input: Result<RelayUserInput, _> = serde_json::from_str(&line);
                        if let Ok(input) = relay_user_input {
                            let dst: PeerId = input.dst.parse().unwrap();
                            swarm.send_message(relay_peer.clone(), dst, input.message.into());
                        } else {
                            println!("incorrect string provided");
                        }
                    }
                    Err(_) => panic!("Stdin closed"),
                }
            },

            // swarm never ends
            from_swarm = swarm.select_next_some() => {
                match from_swarm {
                    InEvent::Relay { src_id, data } => {
                        let peer_id = PeerId::from_bytes(src_id).unwrap();
                        let message = String::from_utf8(data).unwrap();
                        println!("{}: {}", peer_id, message);
                    }
                    InEvent::Upgrade => log::trace!("Upgraded? //TODO: remove that variant")
                }
            })
        }
    })
}
