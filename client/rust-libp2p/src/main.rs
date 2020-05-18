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

use async_std::task;
use ctrlc_adapter::block_until_ctrlc;
use faas_api::{relay, service, FunctionCall};
use fluence_client::{Client, ClientCommand, ClientEvent};
use futures::{
    channel::{mpsc, mpsc::UnboundedReceiver, oneshot},
    prelude::*,
    select,
    stream::StreamExt,
};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use std::error::Error;
use std::time::Duration;

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
                        log::info!("Connected to {}", peer_id);
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

            // blocking happens in 'for' below
            for cmd in stream {
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
        println!("{}", serde_json::to_string_pretty(&cmd).unwrap());
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
            target: Some(service!("IPFS.multiaddr")),
            reply_to: Some(relay!(bootstrap.clone(), peer_id.clone())),
            arguments: json!({ "hash": "QmFile", "msg_id": time }),
            name: Some("call multiaddr".to_string()),
        },
    };

    let register_ipfs_get = ClientCommand::Call {
        node: bootstrap.clone(),
        call: FunctionCall {
            uuid: uuid(),
            target: Some(service!("provide")),
            reply_to: Some(relay!(bootstrap.clone(), peer_id.clone())),
            arguments: json!({ "service_id": "IPFS.get_QmFile3", "msg_id": time }),
            name: Some("register service".to_string()),
        },
    };

    let call_ipfs_get = ClientCommand::Call {
        node: bootstrap.clone(),
        call: FunctionCall {
            uuid: uuid(),
            target: Some(service!("IPFS.get_QmFile3")),
            reply_to: Some(relay!(bootstrap, peer_id.clone())),
            arguments: serde_json::Value::Null,
            name: Some("call ipfs get".to_string()),
        },
    };

    // let call_identify

    println!("Possible messages:");
    println!("\n### call IPFS.multiaddr");
    show(call_multiaddr);
    println!("\n### Register IPFS.get service");
    show(register_ipfs_get);
    println!("\n### Call IPFS.get service");
    show(call_ipfs_get);
    println!("\n")
}
