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

#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod bench_network_models;

use bench_network_models::*;
use connected_client::ConnectedClient;
use connection_pool::ConnectionPoolT;
use control_macro::measure;
use created_swarm::make_swarms_with_mocked_vm;
use itertools::Itertools;
use kademlia::KademliaApiT;
use particle_protocol::Contact;
use std::convert::identity;
use std::time::{Duration, Instant};

#[test]
fn kademlia_resolve() {
    let network_size = 10;

    let (connectivity, _finish_fut, kademlia, peer_ids) =
        connectivity_with_real_kad(1, network_size);

    async_std::task::block_on(async move {
        let start = Instant::now();
        let peer_id = peer_ids.into_iter().skip(3).next().unwrap();
        let result = measure!(connectivity.kademlia.discover_peer(peer_id).await);
        match result {
            Ok(vec) if vec.is_empty() => println!("empty vec!"),
            Err(err) => panic!("err! {}", err),
            Ok(vec) => println!("peer discovered! {:?}", vec),
        }

        measure!(kademlia.cancel().await);
        println!("finished. elapsed {} ms", start.elapsed().as_millis())
    })
}

#[test]
fn connectivity_test() {
    let num_particles = 10;
    let network_size = 5;
    let swarms = make_swarms_with_mocked_vm(network_size, identity, None, identity);
    let first = swarms.iter().next().unwrap();
    let last = swarms.iter().last().unwrap();

    std::thread::sleep(Duration::from_secs(5));

    let sender_node = make_swarms_with_mocked_vm(1, identity, None, identity)
        .into_iter()
        .next()
        .unwrap();
    async_std::task::block_on(
        sender_node
            .connectivity
            .connection_pool
            .connect(Contact::new(first.peer_id, vec![first.multiaddr.clone()])),
    );

    // TODO: it should work without bootstraps, shouldn't it?
    let receiver_node = make_swarms_with_mocked_vm(1, identity, None, |_| vec![])
        .into_iter()
        .next()
        .unwrap();

    async_std::task::block_on(
        receiver_node
            .connectivity
            .connection_pool
            .connect(Contact::new(last.peer_id, vec![last.multiaddr.clone()])),
    );

    let mut receiver_client = ConnectedClient::connect_to(receiver_node.multiaddr.clone())
        .expect("connect receiver_client");

    println!(
        "data = {},{},{}",
        first.peer_id, receiver_node.peer_id, receiver_client.peer_id
    );
    let particles = generate_particles(num_particles, |_, mut p| {
        p.ttl = (u16::MAX - 10) as _;
        p.script = String::from("!");
        p.data = vec![
            first.peer_id,
            receiver_node.peer_id,
            receiver_client.peer_id,
        ]
        .into_iter()
        .map(|p| p.to_string())
        .join(",")
        .into_bytes();
        p
    });

    async_std::task::block_on(async move {
        let contact = Contact::new(sender_node.peer_id, vec![]);
        let num_particles = particles.len();
        for particle in particles {
            sender_node
                .connectivity
                .connection_pool
                .send(contact.clone(), particle);
        }
        for _ in 1..num_particles {
            if let Err(err) = receiver_client.receive() {
                println!("error receiving: {:?}", err);
            }
        }
    })
}
