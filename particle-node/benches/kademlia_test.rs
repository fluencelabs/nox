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

#![allow(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod bench_network_models;
mod tracing_utils;

use bench_network_models::*;
use connection_pool::ConnectionPoolT;
use control_macro::measure;
use kademlia::KademliaApiT;
use particle_protocol::{Contact, ProtocolMessage};
use std::convert::identity;
use std::time::{Duration, Instant};
use test_utils::{enable_logs, make_swarms_with_mocked_vm, ConnectedClient};
use tracing_utils::*;

#[test]
fn kademlia_resolve() {
    let network_size = 100;

    let (connectivity, _finish_fut, kademlia, peer_ids) =
        connectivity_with_real_kad(1, network_size);

    async_std::task::block_on(async move {
        // enable_logs();
        // log::error!("===== test before =====");
        // async_std::task::sleep(Duration::from_secs(1)).await;
        // log::error!("===== test after =====");

        let start = Instant::now();
        let peer_id = peer_ids.into_iter().skip(3).next().unwrap();
        let result = measure!(connectivity.kademlia.discover_peer(peer_id).await);
        match result {
            Ok(vec) if vec.is_empty() => println!("empty vec!"),
            Err(err) => println!("err! {}", err),
            Ok(vec) => println!("peer discovered! {:?}", vec),
        }

        measure!(kademlia.cancel().await);
        println!("finished. elapsed {} ms", start.elapsed().as_millis())
    })
}

#[test]
fn connectivity_test() {
    use control_macro::measure;

    let num_particles = 10;
    let network_size = 10;
    let swarms = make_swarms_with_mocked_vm(network_size, identity, None, identity);
    let first = swarms.iter().next().unwrap();
    let last = swarms.iter().last().unwrap();

    std::thread::sleep(Duration::from_secs(5));
    enable_logs();
    log::error!("===== test before =====");
    std::thread::sleep(Duration::from_secs(1));
    log::error!("===== test after =====");

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

    let receiver_node = make_swarms_with_mocked_vm(1, identity, None, identity)
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
        p.init_peer_id = first.peer_id;
        // p.script = String::from("!");
        // p.data = format!(
        //     "{},{},{}",
        //     first.peer_id, receiver_node.peer_id, receiver_client.peer_id
        // )
        // .into_bytes();
        p
    });

    async_std::task::block_on(async move {
        {
            let bytes = vec![
                123, 34, 97, 99, 116, 105, 111, 110, 34, 58, 34, 80, 97, 114, 116, 105, 99, 108,
                101, 34, 44, 34, 105, 100, 34, 58, 34, 49, 34, 44, 34, 105, 110, 105, 116, 95, 112,
                101, 101, 114, 95, 105, 100, 34, 58, 34, 49, 50, 68, 51, 75, 111, 111, 87, 67, 74,
                104, 76, 98, 78, 51, 118, 67, 101, 112, 109, 70, 106, 114, 87, 70, 53, 90, 70, 68,
                71, 65, 117, 65, 89, 86, 121, 78, 74, 51, 70, 49, 49, 101, 80, 99, 119, 76, 76, 82,
                120, 86, 76, 34, 44, 34, 116, 105, 109, 101, 115, 116, 97, 109, 112, 34, 58, 49,
                54, 49, 55, 55, 51, 55, 48, 49, 54, 57, 51, 49, 44, 34, 116, 116, 108, 34, 58, 54,
                53, 53, 50, 53, 44, 34, 115, 99, 114, 105, 112, 116, 34, 58, 34, 34, 44, 34, 115,
                105, 103, 110, 97, 116, 117, 114, 101, 34, 58, 91, 93, 44, 34, 100, 97, 116, 97,
                34, 58, 34, 34, 125,
            ];
            let test_msg: Result<ProtocolMessage, _> = serde_json::from_slice(&bytes);
            println!("{:?}", test_msg);
            test_msg.unwrap();
        }

        let contact = Contact::new(sender_node.peer_id, vec![]);
        let num_particles = particles.len();
        for particle in particles {
            sender_node
                .connectivity
                .connection_pool
                .send(contact.clone(), particle);
        }
        for _ in 0..num_particles {
            if let Err(err) = receiver_client.receive() {
                println!("error receiving: {:?}", err);
            }
        }
    })
}

#[test]
fn deserialize() {
    let bytes = vec![
        123, 34, 97, 99, 116, 105, 111, 110, 34, 58, 34, 80, 97, 114, 116, 105, 99, 108, 101, 34,
        44, 34, 105, 100, 34, 58, 34, 49, 34, 44, 34, 105, 110, 105, 116, 95, 112, 101, 101, 114,
        95, 105, 100, 34, 58, 34, 49, 50, 68, 51, 75, 111, 111, 87, 67, 74, 104, 76, 98, 78, 51,
        118, 67, 101, 112, 109, 70, 106, 114, 87, 70, 53, 90, 70, 68, 71, 65, 117, 65, 89, 86, 121,
        78, 74, 51, 70, 49, 49, 101, 80, 99, 119, 76, 76, 82, 120, 86, 76, 34, 44, 34, 116, 105,
        109, 101, 115, 116, 97, 109, 112, 34, 58, 49, 54, 49, 55, 55, 51, 55, 48, 49, 54, 57, 51,
        49, 44, 34, 116, 116, 108, 34, 58, 54, 53, 53, 50, 53, 44, 34, 115, 99, 114, 105, 112, 116,
        34, 58, 34, 34, 44, 34, 115, 105, 103, 110, 97, 116, 117, 114, 101, 34, 58, 91, 93, 44, 34,
        100, 97, 116, 97, 34, 58, 34, 34, 125,
    ];
    let test_msg: Result<ProtocolMessage, _> = serde_json::from_slice(&bytes);
    println!("{:?}", test_msg);
    test_msg.unwrap();
}
