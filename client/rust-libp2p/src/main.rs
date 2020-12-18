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

#![feature(stmt_expr_attributes)]
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
use clap::{App, Arg};
use ctrlc_adapter::block_until_ctrlc;
use fluence_client::{Client, ClientCommand, ClientEvent};
use futures::task::Poll;
use futures::{
    channel::{mpsc, mpsc::UnboundedReceiver, oneshot},
    prelude::*,
    select,
    stream::StreamExt,
};
use libp2p::core::Multiaddr;
use libp2p::PeerId;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder().format_timestamp_micros().init();

    let args = &[Arg::from_usage("<multiaddr> 'Multiaddr of the Fluence node'").required(true)];

    let arg_matches = App::new("Connect to fluence server")
        .args(args)
        .get_matches();

    let relay_addr: Multiaddr = arg_matches
        .value_of("multiaddr")
        .expect("multiaddr is required")
        .parse()
        .expect("provided incorrect Multiaddr");

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

    let mut node: Option<PeerId> = None;

    loop {
        select!(
            cmd = stdin_cmds.select_next_some() => {
                match cmd {
                    Ok(cmd) => {
                        if let Some(node) = &node {
                            client.send(cmd.into(), node.clone());
                            print!("\n");
                        } else {
                            print!("Not connected yet!");
                        }
                    },
                    Err(e) => println!("incorrect string provided: {:?}", e)
                }
            },
            incoming = client.receive_one() => {
                match incoming {
                    Some(ClientEvent::NewConnection{ peer_id, ..}) => {
                        log::info!("Connected to {}", peer_id);
                        #[allow(unused_assignments)] // This seems like a linting bug, node is used a few lines above
                        node = Some(peer_id.clone());
                        unimplemented!("print example");
                        /*print_example(Protocol::Peer(peer_id.clone()).into(), client.relay_address(peer_id.clone()));*/
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
            let mut stream = Deserializer::from_reader(stdin).into_iter().fuse();

            let cmd_sender = cmd_sender.clone();
            task::spawn(async move {
                futures::future::poll_fn(|cx| {
                    // Read parsed command from JSON stream (blocking)
                    let cmd = stream.next();
                    if let Some(cmd) = cmd {
                        // Send command to select! that reads from `cmd_recv`
                        cmd_sender.unbounded_send(cmd).expect("send cmd");
                        // Call waker to respawn this future
                        cx.waker().clone().wake();
                        Poll::Pending
                    } else {
                        // Return Ready so await below completes
                        Poll::Ready(())
                    }
                })
                .await;
            })
            .await;
        }
    });

    cmd_recv
}

/*fn print_example(node: Address, reply_to: Address) {
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

    let call_identify = ClientCommand::Particle {
        call: FunctionCall {
            uuid: uuid(),
            target: Some(node),
            reply_to: Some(reply_to.clone()),
            module: Some("provide".into()),
            arguments: json!({ "hash": "QmFile", "msg_id": time }),
            name: Some("call identify".to_string()),
            sender: reply_to,
            ..<_>::default()
        },
    };

    println!("Possible messages:");
    println!("\n### call identify");
    show(call_identify);
    println!("\n")
}
*/
