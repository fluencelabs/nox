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
#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

use async_timer::Interval;
use faas_api::{relay, Address, FunctionCall, Protocol};
use fluence_client::{Client, ClientEvent};
use futures::{channel::oneshot::Receiver, select, FutureExt, StreamExt};
use libp2p::identity::ed25519::Keypair;
use libp2p::PeerId;
use once_cell::sync::Lazy;
use parity_multiaddr::Multiaddr;
use serde_json::json;
use std::error::Error;
use std::ops::Deref;
use std::time::Duration;
use uuid::Uuid;

const IPFS_SERVICE_ID: &str = "IPFS.multiaddr";
#[rustfmt::skip]
static IPFS_SERVICE: Lazy<Protocol> = Lazy::new(|| Protocol::Service(IPFS_SERVICE_ID.to_string()));

fn register_call(client: PeerId, relay: PeerId, service_id: &str, kp: &Keypair) -> FunctionCall {
    let sender = relay!(relay, client, kp);
    let reply_to = Some(sender.clone());
    let target = Some(Protocol::Service("provide".into()).into());
    let arguments = json!({ "service_id": service_id });
    let uuid = message_id();
    let name = Some(format!("Delegate provide service {}", service_id));

    FunctionCall {
        uuid,
        target,
        reply_to,
        arguments,
        name,
        sender,
    }
}

fn multiaddr_call(
    bootstrap_id: PeerId,
    client: PeerId,
    reply_to: Address,
    msg_id: Option<&str>,
    multiaddr: &Multiaddr,
    kp: &Keypair,
) -> FunctionCall {
    let target = Some(reply_to);
    let arguments = json!({ "multiaddr": multiaddr.to_string(), "msg_id": msg_id });
    let sender = relay!(bootstrap_id, client, kp);
    let reply_to = Some(sender.clone());
    let uuid = message_id();
    let name = Some("Reply on IPFS.multiaddr".to_string());

    FunctionCall {
        uuid,
        target,
        reply_to,
        arguments,
        name,
        sender,
    }
}

fn message_id() -> String {
    // TODO: use v1
    Uuid::new_v4().to_string()
}

pub async fn run_ipfs_multiaddr_service(
    bootstrap: Multiaddr,
    ipfs: Multiaddr,
    stop: Receiver<()>,
) -> Result<(), Box<dyn Error>> {
    let (mut client, client_task) = Client::connect(bootstrap.clone()).await?;

    let mut stop = stop.into_stream().fuse();

    let mut bootstrap_id: Option<PeerId> = None;

    // Will publish service 10 times, each 10 seconds
    let mut periodic = Interval::platform_new(Duration::from_secs(10))
        .take(10)
        .fuse();

    loop {
        select!(
            incoming = client.receive_one() => {
                match incoming {
                    Some(ClientEvent::FunctionCall {
                        call: FunctionCall {
                            target: Some(target),
                            reply_to: Some(reply_to),
                            arguments, ..
                        },
                        sender
                    }) if target.contains(&IPFS_SERVICE) => {
                        log::info!(
                            "Got call for {} from {}, asking node to reply to {:?}",
                            IPFS_SERVICE.deref(), sender, reply_to
                        );
                        let msg_id = arguments.get("msg_id").and_then(|v| v.as_str());
                        let call = multiaddr_call(
                            bootstrap_id.clone().unwrap(), client.peer_id.clone(), reply_to, msg_id, &ipfs, &client.key_pair
                        );
                        if let Some(node) = bootstrap_id.clone() {
                            client.send(call, node)
                        } else {
                            log::warn!("Can't send {} reply: bootstrap hasn't connected yed", IPFS_SERVICE.deref());
                        }
                    },
                    Some(ClientEvent::NewConnection { peer_id, multiaddr }) if &multiaddr == &bootstrap => {
                        log::info!("Bootstrap connected, will send register call",);
                        bootstrap_id = Some(peer_id.clone());
                    }
                    Some(msg) => log::info!("Received msg {:?}, ignoring", msg),
                    None => {
                        log::warn!("Client closed");
                        break;
                    }
                }
            },
            _ = periodic.next() => {
                if let Some(bootstrap_id) = bootstrap_id.clone() {
                    let call = register_call(client.peer_id.clone(), bootstrap_id.clone(), IPFS_SERVICE_ID, &client.key_pair);
                    log::info!("Sending register call {:?}", call);

                    client.send(call, bootstrap_id);
                }
            }
            _ = stop.next() => {
                log::info!("Will stop");
                client.stop();
                break;
            }
        )
    }

    log::info!("Waiting client_task");
    client_task.await;
    log::info!("client_task finished, exiting");

    Ok(())
}
