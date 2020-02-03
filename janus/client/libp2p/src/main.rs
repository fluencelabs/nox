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

use crate::behaviour::ClientServiceBehaviour;
use crate::connect_protocol::events::InEvent;
use async_std::{io, task};
use env_logger;
use futures::{future, prelude::*};
use janus_server::peer_service::libp2p::transport::build_transport;
use libp2p::{identity, PeerId};
use parity_multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    error::Error,
    task::{Context, Poll},
    time::Duration,
};

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
        libp2p::Swarm::new(transport, behaviour, local_peer_id.clone())
    };

    let relay_peer_addr: Multiaddr = std::env::args()
        .nth(1)
        .expect("multiaddr of relay peer should be provided by the first argument")
        .parse()
        .expect("provided wrong  Multiaddr");

    match libp2p::Swarm::dial_addr(&mut swarm, relay_peer_addr.clone()) {
        Ok(_) => println!("Dialed to {:?}", relay_peer_addr.clone()),
        Err(e) => {
            println!("Dial to {:?} failed with {:?}", relay_peer_addr.clone(), e);
        }
    }

    let relay_peer: PeerId = std::env::args()
        .nth(2)
        .expect("peer id should be provided by the second argument")
        .parse()
        .expect("provided wrong PeerId");

    let mut stdin = io::BufReader::new(io::stdin()).lines();

    task::block_on(future::poll_fn(move |cx: &mut Context| {
        loop {
            match stdin.try_poll_next_unpin(cx)? {
                Poll::Ready(Some(line)) => {
                    let relay_user_input: Result<RelayUserInput, _> = serde_json::from_str(&line);
                    if let Ok(input) = relay_user_input {
                        let dst: PeerId = input.dst.parse().unwrap();
                        swarm.send_message(relay_peer.clone(), dst, input.message.into());
                    } else {
                        println!("incorrect string provided");
                    }
                }
                Poll::Ready(None) => panic!("Stdin closed"),
                Poll::Pending => break,
            };
        }

        loop {
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(e)) => println!("event received {:?}", e),
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => break,
            }
        }

        if let Some(event) = swarm.pop_out_node_event() {
            match event {
                InEvent::Relay { src_id, data } => {
                    let peer_id = PeerId::from_bytes(src_id).unwrap();
                    let message = String::from_utf8(data).unwrap();
                    println!("{}: {}", peer_id, message);
                }
                InEvent::NetworkState { state } => println!("network state: {:?}", state),
            }
        }

        Poll::Pending
    }))
}
