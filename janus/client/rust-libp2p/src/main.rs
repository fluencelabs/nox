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

use async_std::task;
use ctrlc_adapter::block_until_ctrlc;
use faas_api::{Address, FunctionCall};
use failure::_core::time::Duration;
use futures::{
    channel::{mpsc, mpsc::UnboundedReceiver, oneshot},
    prelude::*,
    select,
    stream::StreamExt,
};
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
    exit_sender
        .send(())
        .expect("send exit signal to client task");
    task::block_on(client_task);

    Ok(())
}

async fn run_client(
    exit_receiver: oneshot::Receiver<()>,
    relay: Multiaddr,
) -> Result<(), Box<dyn Error>> {
    let client = Client::connect(relay);
    let (mut client, client_task) = client.await?;

    let stdin_cmds = read_cmds_from_stdin();

    let mut stdin_cmds = stdin_cmds.into_stream().fuse();
    let mut stop = exit_receiver.into_stream().fuse();

    loop {
        select!(
            cmd = stdin_cmds.select_next_some() => {
                match cmd {
                    Ok(cmd) => {
                        client.send(cmd);
                        print!("\n");
                    },
                    Err(e) => println!("incorrect string provided: {:?}", e)
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

// TODO: it's not clear how and why this all works, so it is possible
//       that it would BLOCK ALL THE FUTURES in the executor, so
//       BE CAREFUL
fn read_cmds_from_stdin() -> UnboundedReceiver<serde_json::error::Result<ClientCommand>> {
    let (cmd_sender, cmd_recv) = mpsc::unbounded();
    task::spawn(async move {
        use serde_json::Deserializer;
        use std::io; // NOTE: this is synchronous IO

        loop {
            let stdin = io::BufReader::new(io::stdin());
            let stream = Deserializer::from_reader(stdin).into_iter::<ClientCommand>();

            for cmd in stream {
                // blocking happens in 'for'
                cmd_sender.unbounded_send(cmd).expect("send cmd");
                task::sleep(Duration::from_nanos(10)).await; // return Poll::Pending from future's fn poll
            }
        }
    });

    cmd_recv
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
    println!("\n### call IPFS.multiaddr");
    show(call_multiaddr);
    println!("\n### Register IPFS.get service");
    show(register_ipfs_get);
    println!("\n### Call IPFS.get service");
    show(call_ipfs_get);
    println!("\n")
}
