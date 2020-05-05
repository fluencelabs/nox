/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
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

use async_std::{io, task};
use ctrlc_adapter::block_until_ctrlc;
use faas_api::{Address, FunctionCall};
use futures::{channel::oneshot, prelude::*, select, stream::StreamExt};
use janus_client::{Client, ClientCommand, ClientEvent};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder().format_timestamp_micros().init();

    let relay_addr: Multiaddr = std::env::args()
        .nth(1)
        .expect("multiaddr of relay peer should be provided by the first argument")
        .parse()
        .expect("provided wrong  Multiaddr");

    let (exit_sender, exit_receiver) = oneshot::channel::<()>();

    let client_task = task::spawn(async move {
        run_client(exit_receiver, relay_addr)
            .await
            .expect("Error running client"); // TODO: handle errors
    });

    block_until_ctrlc();
    exit_sender.send(()).unwrap();
    task::block_on(client_task);

    Ok(())
}

async fn run_client(
    exit_receiver: oneshot::Receiver<()>,
    relay: Multiaddr,
) -> Result<(), Box<dyn Error>> {
    let client = Client::connect(relay);
    let (mut client, client_task) = client.await?;

    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();

    let mut stop = exit_receiver.into_stream().fuse();

    loop {
        select!(
            from_stdin = stdin.select_next_some() => {
                match from_stdin {
                    Ok(line) => {
                        let cmd: Result<ClientCommand, _> = serde_json::from_str(&line);
                        if let Ok(cmd) = cmd {
                            client.send(cmd);
                            print!("\n");
                        } else {
                            println!("incorrect string provided\n");
                        }
                    }
                    Err(_) => panic!("Stdin closed"),
                }
            },
            incoming = client.receive_one() => {
                match incoming {
                    Some(ClientEvent::NewConnection{ peer_id, ..}) => {
                        log::info!("Connected to {}", peer_id.to_base58());
                        print_example(&client.peer_id, peer_id);
                    }
                    Some(msg) => println!("Received\n{}\n", serde_json::to_string_pretty(&msg).unwrap()),
                    None => {
                        println!("Client closed");
                        break;
                    }
                }
            },
            _ = stop.next() => {
                client.stop();
                break;
            }
        )
    }

    client_task.await;

    Ok(())
}

fn print_example(peer_id: &PeerId, bootstrap: PeerId) {
    use serde_json::json;
    use std::time::SystemTime;
    fn show(cmd: ClientCommand) {
        println!("{}", serde_json::to_value(cmd).unwrap());
    }
    fn uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string();

    let call_multiaddr = ClientCommand::Call {
        node: bootstrap.clone(),
        call: FunctionCall {
            uuid: uuid(),
            target: Some(Address::Service {
                service_id: "IPFS.multiaddr".into(),
            }),
            reply_to: Some(Address::Relay {
                relay: bootstrap.clone(),
                client: peer_id.clone(),
            }),
            arguments: json!({ "hash": "QmFile", "msg_id": time }),
            name: Some("call multiaddr".to_string()),
        },
    };

    let register_ipfs_get = ClientCommand::Call {
        node: bootstrap.clone(),
        call: FunctionCall {
            uuid: uuid(),
            target: Some(Address::Service {
                service_id: "provide".into(),
            }),
            reply_to: Some(Address::Relay {
                relay: bootstrap.clone(),
                client: peer_id.clone(),
            }),
            arguments: json!({ "service_id": "IPFS.get_QmFile3", "msg_id": time }),
            name: Some("register service".to_string()),
        },
    };

    let call_ipfs_get = ClientCommand::Call {
        node: bootstrap.clone(),
        call: FunctionCall {
            uuid: uuid(),
            target: Some(Address::Service {
                service_id: "IPFS.get_QmFile3".into(),
            }),
            reply_to: Some(Address::Relay {
                relay: bootstrap,
                client: peer_id.clone(),
            }),
            arguments: serde_json::Value::Null,
            name: Some("call ipfs get".to_string()),
        },
    };

    println!("possible messages:");
    show(call_multiaddr);
    show(register_ipfs_get);
    show(call_ipfs_get);
    println!("\n")
}
