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

use futures::prelude::*;
use janus_server::node_service::behaviour::NodeServiceBehaviour;
use janus_server::node_service::transport;
use libp2p::{
    identity,
    tokio_codec::{FramedRead, LinesCodec},
    PeerId, Swarm,
};
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
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let relay_example = RelayUserInput {
        dst: "QmNm9NAew7oqitRRP1efai7g1nuvYonSpyBAb2hW5DEqNs".to_string(),
        message: "hello".to_string(),
    };
    let relay_example = serde_json::to_value(relay_example).unwrap();
    println!("example of a relay message: {}", relay_example.to_string());

    let transport = transport::build_transport(local_key.clone(), Duration::from_secs(20));

    let mut swarm = {
        let behaviour = NodeServiceBehaviour::new(&local_peer_id, local_key.public());
        libp2p::Swarm::new(transport, behaviour, local_peer_id.clone())
    };

    // Reach out to another node if specified
    if let Some(to_dial) = std::env::args().nth(1) {
        let dialing = to_dial.clone();
        match to_dial.parse() {
            Ok(to_dial) => match libp2p::Swarm::dial_addr(&mut swarm, to_dial) {
                Ok(_) => println!("Dialed {:?}", dialing),
                Err(e) => println!("Dial {:?} failed: {:?}", dialing, e),
            },
            Err(err) => println!("Failed to parse address to dial: {:?}", err),
        }
    }

    let stdin = tokio::io::stdin();
    let mut framed_stdin = FramedRead::new(stdin, LinesCodec::new());

    libp2p::Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/7779".parse().unwrap()).unwrap();

    // Kick it off
    let mut listening = false;
    tokio::run(futures::future::poll_fn(move || -> Result<_, ()> {
        loop {
            match framed_stdin.poll().expect("Error while polling stdin") {
                Async::Ready(Some(line)) => {
                    let relay_user_input: Result<RelayUserInput, _> = serde_json::from_str(&line);
                    if let Ok(input) = relay_user_input {
                        let dst: PeerId = input.dst.parse().unwrap();
                        swarm.node_connect_protocol.relay_message(
                            local_peer_id.clone(),
                            dst,
                            input.message.into(),
                        );
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
                    if !listening {
                        if let Some(a) = Swarm::listeners(&swarm).next() {
                            println!("Listening on {:?}", a);
                            listening = true;
                        }
                    }
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
