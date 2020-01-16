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

#[allow(dead_code)]

mod behaviour;
mod connect_protocol;
mod transport;

use crate::behaviour::ClientServiceBehaviour;
use crate::transport::build_transport;
use env_logger;
use futures::prelude::*;
use libp2p::{
    identity,
    tokio_codec::{FramedRead, LinesCodec},
    PeerId,
};
use parity_multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::Duration;

// user input for relaying (just a json now)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct RelayUserInput {
    dst: String,
    message: String,
}

fn main() {
    env_logger::init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let relay_example = RelayUserInput {
        dst: "QmNm9NAew7oqitRRP1efai7g1nuvYonSpyBAb2hW5DEqNs".to_string(),
        message: "hello".to_string(),
    };
    let relay_example = serde_json::to_value(relay_example).unwrap();
    println!("example of a relay message: {}", relay_example.to_string());

    let mut swarm = {
        let transport = build_transport(local_key.clone(), Duration::from_secs(20));
        let behaviour = ClientServiceBehaviour::new(&local_peer_id, local_key.public());
        libp2p::Swarm::new(transport, behaviour, local_peer_id.clone())
    };

    let peer_to_dial: Multiaddr = std::env::args()
        .nth(1)
        .expect("multiaddr of relay peer should be provided by the first argument")
        .parse()
        .expect("provided wrong  Multiaddr");

    match libp2p::Swarm::dial_addr(&mut swarm, peer_to_dial.clone()) {
        Ok(_) => println!("Dialed to {:?}", peer_to_dial),
        Err(e) => {
            println!("Dial to {:?} failed with {:?}", peer_to_dial, e);
            return;
        }
    }

    let connected_peer: PeerId = std::env::args()
        .nth(2)
        .expect("peer id should be provided by the second argument")
        .parse()
        .expect("provided wrong PeerId");

    let stdin = tokio_stdin_stdout::stdin(0);
    let mut framed_stdin = FramedRead::new(stdin, LinesCodec::new());

    tokio::run(futures::future::poll_fn(move || -> Result<_, ()> {
        loop {
            match framed_stdin.poll().expect("Error while polling stdin") {
                Async::Ready(Some(line)) => {
                    let relay_user_input: Result<RelayUserInput, _> = serde_json::from_str(&line);
                    if let Ok(input) = relay_user_input {
                        let dst: PeerId = input.dst.parse().unwrap();
                        swarm.send_message(connected_peer.clone(), dst, input.message.into());
                    } else {
                        println!("incorrect string provided");
                    }
                }
                Async::Ready(None) => panic!("Stdin closed"),
                Async::NotReady => break,
            };
        }

        loop {
            match swarm.poll().expect("Error while polling swarm") {
                Async::Ready(Some(_)) => {}
                Async::Ready(None) | Async::NotReady => {
                    break;
                }
            }
        }

        if let Some(event) = swarm.pop_out_node_event() {
            println!("{:?}", event);
        }

        Ok(Async::NotReady)
    }));
}
