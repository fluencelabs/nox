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

mod config;
mod error;
mod node_service;
mod peer_service;

use crate::node_service::node_service::{start_node_service, NodeService, NodeServiceDescriptor};
use crate::peer_service::peer_service::{start_peer_service, PeerService};
use std::thread;
use std::time;
use tokio;

fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio Runtime");

    let node_service = NodeService::new(config::NodeServiceConfig::default());
    let node_service_descriptor: NodeServiceDescriptor =
        start_node_service(node_service, &runtime.executor())
            .expect("An error occurred during node service start");

    let peer_service = PeerService::new(config::PeerServiceConfig::default());
    let peer_service_exit = start_peer_service(
        peer_service,
        node_service_descriptor.node_channel_out,
        node_service_descriptor.node_channel_in,
        &runtime.executor(),
    )
    .expect("An error occurred during the peer service start");

    println!("Janus has been successfully started");
    let ten_millis = time::Duration::from_secs(120);
    thread::sleep(ten_millis);

    println!("exiting");
    node_service_descriptor
        .exit_sender
        .send(())
        .expect("failed Janus exiting");
    peer_service_exit.send(()).expect("failed Janus exiting");
}
