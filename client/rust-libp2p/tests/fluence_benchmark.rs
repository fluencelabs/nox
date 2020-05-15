// /*
//  * Copyright 2020 Fluence Labs Limited
//  *
//  * Licensed under the Apache License, Version 2.0 (the "License");
//  * you may not use this file except in compliance with the License.
//  * You may obtain a copy of the License at
//  *
//  *     http://www.apache.org/licenses/LICENSE-2.0
//  *
//  * Unless required by applicable law or agreed to in writing, software
//  * distributed under the License is distributed on an "AS IS" BASIS,
//  * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  * See the License for the specific language governing permissions and
//  * limitations under the License.
//  */
//
// #![recursion_limit = "1024"]
//
// pub mod constants;
//
// use crate::constants::NODES;
//
// use fluence_client::client::Client;
//
// use async_std::future::timeout;
// use async_std::task;
// use futures::channel::mpsc;
// use futures::select;
// use futures::stream::{FuturesUnordered, StreamExt};
//
// use bencher::stats::Stats;
// use futures::channel::mpsc::TrySendError;
// use itertools::Itertools;
// use fluence_client::ClientEvent;
// use libp2p::PeerId;
// use parity_multiaddr::Multiaddr;
// use std::error::Error;
// use std::time::{Duration, SystemTime};
//
// const MSG_COUNT: u32 = 5u32;
//
// #[derive(Clone)]
// struct Node {
//     address: Multiaddr,
//     peer_id: PeerId,
// }
//
// impl Node {
//     fn new(address: Multiaddr, peer_id: PeerId) -> Self {
//         Node { address, peer_id }
//     }
// }
//
// fn nodes() -> Vec<Node> {
//     NODES
//         .iter()
//         .map(|(address, peer_id)| Node::new(address.parse().unwrap(), peer_id.parse().unwrap()))
//         .collect()
// }
//
// #[allow(dead_code)]
// fn local_nodes() -> Vec<Node> {
//     vec![
//         Node::new(
//             "/ip4/127.0.0.1/tcp/9990/ws".parse().unwrap(),
//             "QmY28NSCefB532XbERtnKHadexGuNzAfYnh5fJk6qhLsSi"
//                 .parse()
//                 .unwrap(),
//         ),
//         Node::new(
//             "/ip4/127.0.0.1/tcp/9991/ws".parse().unwrap(),
//             "Qme4XRbTMzYax1NyKp4dR4NS1bcVkF1Xw3fpWE8J1eCyRe"
//                 .parse()
//                 .unwrap(),
//         ),
//     ]
// }
//
// type TaskId = usize;
//
// #[derive(Clone, Debug)]
// enum Action {
//     Sent,
//     Received(Duration),
//     Finished,
//     Started,
//     Iterated(TaskId, u32),
//     Timeout,
// }
//
// // cargo test -- --nocapture
// #[test]
// pub fn measure_relay_test() {
//     task::block_on(run_measures()).unwrap()
// }
//
// async fn run_measures() -> Result<(), Box<dyn Error>> {
//     let (stat_outlet, stat_inlet) = mpsc::unbounded();
//     let clients = nodes()
//         .into_iter()
//         // Take all possible 2-combinations of nodes
//         .tuple_combinations()
//         // .take(1)
//         // .tuples::<(_, _)>() // uncomment to take only pairs
//         // Create & connect clients to a corresponding nodes
//         .map(|(a, b)| connect_clients(a, b))
//         .collect::<FuturesUnordered<_>>()
//         .collect::<Vec<_>>();
//
//     // Convert Vec of Results to Result of Vec
//     let mut clients: Vec<_> = clients.await.into_iter().collect::<Result<_, _>>()?;
//     println!(
//         "All clients connected. Total pairs count: {}",
//         clients.len()
//     );
//
//     // TODO: it works only when there's a pause between connection and message sending :( Why?
//     task::sleep(Duration::from_millis(1000)).await;
//
//     // convert client pairs to the measure tasks
//     let futures: FuturesUnordered<_> = clients
//         .iter_mut()
//         .enumerate()
//         // Asynchronously run measuring task
//         .map(|(i, (a, b))| run_measure(i, a, b, MSG_COUNT, stat_outlet.clone()))
//         .collect();
//     println!("Spawned.");
//
//     drop(stat_outlet);
//
//     // TODO: stats outlet is dropped before all events are received by inlet
//     //       https://book.async.rs/tutorial/handling_disconnection.html
//     let mut stats = stat_inlet.fuse();
//     let mut measures: Vec<Action> = vec![];
//     // Stream of tasks
//     let mut tasks = futures.fuse();
//
//     async move {
//         loop {
//             select!(
//                 stat = stats.next() => {
//                     match stat {
//                         Some(s) => {
//                             measures.push(s.clone());
//                             match s {
//                                 Action::Iterated(id, i) => {}, // println!("task {} iteration {}", id, i),
//                                 _ => {}
//                             }
//                         }
//                         None => {
//                             // When stats channel is closed, print measures one last time
//                             print_measures(&measures);
//                             // No more stats writers => break the loop
//                             println!("All stats received.");
//                             break
//                         }
//                     }
//                 }
//
//                 task = tasks.next() => {
//                     match task {
//                         // print measures every time a task is finished
//                         Some(Ok(_)) => print_measures(&measures),
//                         Some(Err(e)) => eprintln!("Task finished with error {:?}", e),
//                         None => {
//                             println!("All tasks finished.")
//                         }
//                     }
//                 }
//             )
//         }
//     }
//     .await;
//     Ok(())
// }
//
// /// Creates 2 clients and connects them to nodes correspondingly
// async fn connect_clients(node1: Node, node2: Node) -> Result<(Client, Client), Box<dyn Error>> {
//     let (client1, _) = Client::connect(node1.address).await?;
//     let (client2, _) = Client::connect(node2.address).await?;
//     Ok((client1, client2))
// }
//
// async fn run_measure(
//     id: TaskId,
//     client1: &Client,
//     client2: &mut Client,
//     count: u32,
//     stat: mpsc::UnboundedSender<Action>,
// ) -> Result<(), TrySendError<Action>> {
//     stat.unbounded_send(Action::Started)?;
//
//     for i in 0..count {
//         send_and_wait(client1, client2, &stat).await?;
//         stat.unbounded_send(Action::Iterated(id, i))?;
//     }
//
//     stat.unbounded_send(Action::Finished)?;
//
//     Ok(())
// }
//
// async fn send_and_wait(
//     client1: &Client,
//     client2: &mut Client,
//     stat: &mpsc::UnboundedSender<Action>,
// ) -> Result<(), TrySendError<Action>> {
//     client1.send(Command::Relay {
//         dst: client2.peer_id.clone(),
//         data: now().as_millis().to_string(),
//     });
//
//     stat.unbounded_send(Action::Sent)?;
//
//     // TODO: move timeout to arguments
//     let result = timeout(Duration::from_secs(5), client2.receive_one()).await;
//     let now = now();
//
//     match result {
//         Ok(Some(ClientEvent::FunctionCall { call, sender })) => {
//             let sent = call
//                 .arguments
//                 .get("sent")
//                 .and_then(|v| v.as_u64())
//                 .map(Duration::from_millis)
//                 .expect("parse 'sent' from arguments");
//
//             let passed = now - sent;
//             stat.unbounded_send(Action::Received(passed))?;
//         }
//         Err(_) => stat.unbounded_send(Action::Timeout)?,
//         _ => {}
//     }
//
//     Ok(())
// }
//
// fn now() -> Duration {
//     std::time::SystemTime::now()
//         .duration_since(SystemTime::UNIX_EPOCH)
//         .expect("Error on now()")
// }
//
// fn print_measures(measures: &Vec<Action>) {
//     fn pp(f: f64) -> u128 {
//         Duration::from_secs_f64(f).as_millis()
//     }
//
//     let received: Vec<_> = measures
//         .iter()
//         .filter_map(|m| match m {
//             Action::Received(d) => Some(d.as_secs_f64()),
//             _ => None,
//         })
//         .collect();
//
//     let (mut sent, mut started, mut finished, mut timeout) = (0u32, 0u32, 0u32, 0u32);
//     for m in measures {
//         match m {
//             Action::Sent => sent += 1,
//             Action::Started => started += 1,
//             Action::Finished => finished += 1,
//             Action::Timeout => timeout += 1,
//             _ => {}
//         }
//     }
//
//     println!(
//         "\n\nstartd\tfinshd\tsent\trcvd\ttimeout\n{}\t{}\t{}\t{}\t{}",
//         started,
//         finished,
//         sent,
//         received.len(),
//         timeout,
//     );
//
//     if !received.is_empty() {
//         println!(
//             "mean\tmedian\tvar\t.75\t.95\t.99\tmax\tmin\n{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
//             pp(received.mean()),
//             pp(received.median()),
//             pp(received.var()),
//             pp(received.percentile(75.0)),
//             pp(received.percentile(95.0)),
//             pp(received.percentile(99.0)),
//             pp(received.max()),
//             pp(received.min())
//         );
//     }
// }
