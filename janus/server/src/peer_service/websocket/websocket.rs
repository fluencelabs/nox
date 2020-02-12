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

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures_util::future::FutureExt;
use log::{error, info, trace};
use std::str::FromStr;

use crate::peer_service::websocket::events::WebsocketEvent;
use futures::channel::{mpsc, oneshot};

use tungstenite::handshake::server::{ErrorResponse, Request};
use tungstenite::http::StatusCode;

use libp2p::PeerId;

use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future, pin_mut, select,
    stream::StreamExt,
    stream::TryStreamExt,
};

use crate::config::WebsocketConfig;
use crate::peer_service::libp2p::notifications::{InPeerNotification, OutPeerNotification};
use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use tungstenite::protocol::Message;

type ConnectionMap = Arc<Mutex<HashMap<PeerId, UnboundedSender<Message>>>>;

/// Gets peerId from url path and registers a handler for incoming messages
async fn handle_websocket_connection(
    peer_map: ConnectionMap,
    raw_stream: TcpStream,
    peer_channel_in: mpsc::UnboundedSender<OutPeerNotification>,
) -> Result<(), ()> {
    let (peer_id_sender, peer_id_receiver) = oneshot::channel();

    // callback to parse the incoming request, gets peerId from the path
    let callback = |req: &Request| {
        trace!("Received a new ws handshake");
        trace!("The request's path is: {}", req.path);

        // todo
        let index = match req.path.find("key=") {
            None => {
                let status_code = StatusCode::from_u16(500).unwrap();
                let err = ErrorResponse {
                    error_code: status_code,
                    body: None,
                    headers: None,
                };
                return Err(err);
            }
            Some(i) => i,
        };

        // size of 'key='
        let split_size = 4;
        //todo
        let key = req.path.split_at(index + split_size).1;

        let key: PeerId = match PeerId::from_str(key) {
            Err(e) => {
                let status_code = StatusCode::from_u16(500).unwrap();
                let err = ErrorResponse {
                    error_code: status_code,
                    body: Some(format!("Cannot parse key {}: {}", key, e)),
                    headers: None,
                };
                return Err(err);
            }
            Ok(peer) => peer,
        };

        peer_id_sender.send(key).unwrap();

        Ok(None)
    };

    let ws_stream = async_tungstenite::accept_hdr_async(raw_stream, callback)
        .await
        .map_err(|_| error!("Error during the websocket handshake occurred"))?;

    let peer_id = peer_id_receiver
        .await
        .map_err(|_| error!("Cannot get peer_id during the websocket handshake occurred"))?;

    info!("WebSocket connection established: {}", peer_id);

    // insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(peer_id.clone(), tx);

    let (outgoing, incoming) = ws_stream.split();

    peer_channel_in
        .unbounded_send(OutPeerNotification::PeerConnected {
            peer_id: peer_id.clone(),
        })
        .unwrap();

    let broadcast_incoming =
        incoming.try_for_each(|msg| handle_message(msg, peer_id.clone(), peer_channel_in.clone()));

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    peer_channel_in
        .unbounded_send(OutPeerNotification::PeerDisconnected {
            peer_id: peer_id.clone(),
        })
        .unwrap();

    info!("{} disconnected", peer_id);
    peer_map.lock().unwrap().remove(&peer_id);
    Ok(())
}

/// Handles incoming messages from websocket
fn handle_message(
    msg: tungstenite::Message,
    self_peer_id: PeerId,
    peer_channel_in: mpsc::UnboundedSender<OutPeerNotification>,
) -> impl futures::Future<Output = Result<(), tungstenite::error::Error>> {
    let text = match msg.to_text() {
        Ok(r) => r,
        Err(e) => return future::err(e),
    };

    trace!("Received a message from {}: {}", self_peer_id, text);

    let websocket_event: WebsocketEvent = match serde_json::from_str(text) {
        Err(_) => {
            info!("Cannot parse message: {}", text);
            return future::ok(());
        }
        Ok(v) => v,
    };

    match websocket_event {
        WebsocketEvent::Relay { peer_id, data } => {
            let dst_peer_id = PeerId::from_str(peer_id.as_str()).unwrap();
            let msg = OutPeerNotification::Relay {
                src_id: self_peer_id,
                dst_id: dst_peer_id,
                data: data.into_bytes(),
            };
            peer_channel_in.unbounded_send(msg).unwrap();
        }
        WebsocketEvent::GetNetworkState => {
            let msg = OutPeerNotification::GetNetworkState {
                src_id: self_peer_id,
            };
            peer_channel_in.unbounded_send(msg).unwrap();
        }
        m => trace!("Unexpected event has been received: {:?}", m),
    }

    future::ok(())
}

/// Handles libp2p notifications from the node service
fn handle_node_service_notification(event: InPeerNotification, peer_map: ConnectionMap) {
    match event {
        InPeerNotification::Relay {
            src_id,
            dst_id,
            data,
        } => {
            let peers = peer_map.lock().unwrap();
            let recipient = peers
                .iter()
                .find(|(peer_addr, _)| peer_addr == &&dst_id)
                .map(|(_, ws_sink)| ws_sink);

            if let Some(recp) = recipient {
                let msg = WebsocketEvent::Relay {
                    peer_id: src_id.to_base58(),
                    data: String::from_utf8(data).unwrap(),
                };
                let msg = serde_json::to_string(&msg).unwrap();
                let msg = tungstenite::protocol::Message::Text(msg);
                recp.unbounded_send(msg).unwrap();
            };
        }

        InPeerNotification::NetworkState { dst_id, state } => {
            let peers = peer_map.lock().unwrap();
            let recipient = peers
                .iter()
                .find(|(peer_addr, _)| peer_addr == &&dst_id)
                .map(|(_, ws_sink)| ws_sink);

            if let Some(recp) = recipient {
                let msg = WebsocketEvent::NetworkState {
                    peers: state.iter().map(|p| p.to_base58()).collect(),
                };
                let msg = serde_json::to_string(&msg).unwrap();
                let msg = tungstenite::protocol::Message::Text(msg);
                recp.unbounded_send(msg).unwrap();
            };
        }
    }
}

/// Binds port to establish websocket connections, runs peer service based on websocket
pub fn start_peer_service(
    config: WebsocketConfig,
    peer_channel_in: mpsc::UnboundedReceiver<InPeerNotification>,
    peer_channel_out: mpsc::UnboundedSender<OutPeerNotification>,
) -> oneshot::Sender<()> {
    let addr = format!("{}:{}", config.listen_ip, config.listen_port);

    trace!("binding address for websocket");

    let try_socket = task::block_on(TcpListener::bind(&addr));
    let listener = try_socket.expect("Failed to bind");

    let peer_map = ConnectionMap::new(Mutex::new(HashMap::new()));

    let (exit_sender, exit_receiver) = oneshot::channel();

    // Create the event loop and TCP listener we'll accept connections on.
    task::spawn(async move {
        //fusing streams
        let mut incoming = listener.incoming().fuse();
        let mut peer_channel_in = peer_channel_in.fuse();
        let mut exit_receiver = exit_receiver.into_stream().fuse();

        loop {
            select! {
                from_socket = incoming.next() => {
                    match from_socket {
                        Some(Ok(stream)) => {
                            // spawn a separate async thread for each incoming connection
                            task::spawn(handle_websocket_connection(
                                peer_map.clone(),
                                stream,
                                peer_channel_out.clone(),
                            ));
                        },

                        Some(Err(e)) =>
                            println!("Error while receiving incoming connection: {:?}", e),

                        None => {
                            error!("websocket/select: incoming has unexpectedly closed");

                            // socket is closed - break the loop
                            break;
                        }
                    }
                },

                from_node = peer_channel_in.next() => {
                    match from_node {
                        Some(notification) => handle_node_service_notification(
                            notification,
                            peer_map.clone()
                        ),

                        // channel is closed when node service was shut down - break the loop
                        None => break,
                    }
                },

                _ = exit_receiver.next() => {
                    break;
                },
            }
        }
    });

    exit_sender
}
