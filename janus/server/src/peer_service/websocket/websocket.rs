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

use faster_hex::hex_decode;
use sodiumoxide::crypto;
use tungstenite::handshake::server::{ErrorResponse, Request};
use tungstenite::http::StatusCode;

use libp2p::PeerId;

use futures::{channel::mpsc::UnboundedSender, future, pin_mut, select, stream::StreamExt};

use crate::config::config::WebsocketPeerServiceConfig;
use crate::peer_service::libp2p::events::{ToNodeMsg, ToPeerMsg};
use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use std::error::Error;
use tungstenite::protocol::Message;

type ConnectionMap = Arc<Mutex<HashMap<PeerId, UnboundedSender<Message>>>>;

/// Gets peerId from url path and registers a handler for incoming messages
async fn handle_websocket_connection(
    peer_map: ConnectionMap,
    raw_stream: TcpStream,
    node_outlet: mpsc::UnboundedSender<ToNodeMsg>,
) -> Result<(), ()> {
    let (ws_peer_id_outlet, ws_peer_id_inlet) = oneshot::channel();

    // TODO: move to a separate function
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

        ws_peer_id_outlet.send(key).unwrap();

        Ok(None)
    };

    let ws_stream = async_tungstenite::accept_hdr_async(raw_stream, callback)
        .await
        .map_err(|e| error!("Error during the websocket handshake occurred: {}", e))?;
    let (to_ws_sink, ws_stream) = ws_stream.split();

    // PeerId of the connected peer
    let ws_peer_id = ws_peer_id_inlet
        .await
        .map_err(|_| error!("Cannot get peer_id during the websocket handshake occurred"))?;

    info!("WebSocket connection established with {}", ws_peer_id);

    // channel for sending messages to peer's (client's) websocket
    let (ws_outlet, ws_inlet) = mpsc::unbounded();

    // insert the write part of this peer to the peer map.
    peer_map
        .lock()
        .unwrap()
        .insert(ws_peer_id.clone(), ws_outlet.clone());

    node_outlet
        .unbounded_send(ToNodeMsg::PeerConnected {
            peer_id: ws_peer_id.clone(),
        })
        .unwrap();

    let processing_fut = ws_stream
        .map(parse_message)
        .map(filter_by_signature)
        .map(|m| convert(m, ws_peer_id.clone()))
        .map(|msg| match msg {
            Ok(msg) => node_outlet.unbounded_send(msg).map_err(box_err),
            Err(err) => ws_outlet
                .unbounded_send(to_ws_message(err))
                .map_err(box_err),
        })
        .into_future();

    let send_to_ws_fut = ws_inlet.map(Ok).forward(to_ws_sink);

    pin_mut!(processing_fut, send_to_ws_fut);
    future::select(processing_fut, send_to_ws_fut).await;

    node_outlet
        .unbounded_send(ToNodeMsg::PeerDisconnected {
            peer_id: ws_peer_id.clone(),
        })
        .unwrap();

    info!("{} disconnected", ws_peer_id);
    peer_map.lock().unwrap().remove(&ws_peer_id);
    Ok(())
}

fn to_ws_message(event: WebsocketEvent) -> tungstenite::protocol::Message {
    let msg = serde_json::to_string(&event).unwrap();
    tungstenite::protocol::Message::Text(msg)
}

fn hex_decode_str(hex: &str) -> Result<Vec<u8>, String> {
    let hex_bytes = hex.as_bytes();
    if hex_bytes.len() % 2 != 0 {
        return Err("Incorrect hex length. Must be a multiple of 2.".to_string());
    }
    let mut bytes = vec![0; hex_bytes.len() / 2];
    match hex_decode(hex_bytes, &mut bytes) {
        Ok(_) => Ok(bytes),
        Err(err) => Err(format!("{}", err)),
    }
}

/// Check that signature for data is correct for current public key
// todo: check that hash of data is equals with the signature data part
fn check_signature(pk_hex: &str, signature_hex: &str, _data: &str) -> bool {
    let pk_bytes = match hex_decode_str(pk_hex) {
        Ok(b) => b,
        Err(err_msg) => {
            info!("Error on decoding public key: {}", err_msg);
            return false;
        }
    };
    let mut pk = crypto::sign::PublicKey([0u8; 32]);
    pk.0.copy_from_slice(&pk_bytes);

    let signature = match hex_decode_str(signature_hex) {
        Ok(b) => b,
        Err(err_msg) => {
            info!("Error on decoding signature: {}", err_msg);
            return false;
        }
    };

    crypto::sign::verify(&signature, &pk).is_ok()
}

fn box_err<E: Error + 'static>(e: E) -> Box<dyn Error> {
    Box::new(e)
}

fn parse_message(r: Result<tungstenite::Message, tungstenite::Error>) -> WebsocketEvent {
    let event = r.map_err(box_err);
    let event = event.and_then(|msg| msg.to_text().map(|s| s.to_string()).map_err(box_err));
    let event = event.and_then(|msg| serde_json::from_str(msg.as_str()).map_err(box_err));

    match event {
        Err(err) => {
            let err_msg = format!("Cannot parse message: {:?}", err);
            WebsocketEvent::Error(err_msg)
        }
        Ok(v) => v,
    }
}

fn filter_by_signature(msg: WebsocketEvent) -> WebsocketEvent {
    match msg {
        WebsocketEvent::Relay {
            data,
            p_key,
            signature,
            ..
        } if !check_signature(&p_key, &signature, &data) => {
            // signature check failed - send error and exit from the handler
            let err_msg = "Signature does not match message.";
            WebsocketEvent::Error(err_msg.to_string())
        }
        _ => msg,
    }
}

fn convert(msg: WebsocketEvent, self_peer_id: PeerId) -> Result<ToNodeMsg, WebsocketEvent> {
    if let WebsocketEvent::Relay { peer_id, data, .. } = msg {
        let dst_peer_id: PeerId = peer_id.parse().map_err(|e| {
            WebsocketEvent::Error(format!("Cannot parse dst_peer_id {}: {}", peer_id, e))
        })?;

        Ok(ToNodeMsg::Relay {
            src_id: self_peer_id,
            dst_id: dst_peer_id,
            data: data.into_bytes(),
        })
    } else {
        Err(msg)
    }
}

/// Handles libp2p events from the node service
fn handle_node_to_peer_event(event: ToPeerMsg, peer_map: ConnectionMap) {
    match event {
        ToPeerMsg::Deliver {
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
                let event = WebsocketEvent::Relay {
                    peer_id: src_id.to_base58(),
                    data: String::from_utf8(data).unwrap(),
                    p_key: "".to_string(),
                    signature: "".to_string(),
                };
                let msg = to_ws_message(event);
                recp.unbounded_send(msg).unwrap();
            };
        }
    }
}

/// Binds port to establish websocket connections, runs peer service based on websocket
/// * `peer_outlet` – channel to receive events from node service
/// * `node_outlet` – channel to send events to node service
pub fn start_peer_service(
    config: WebsocketPeerServiceConfig,
    peer_outlet: mpsc::UnboundedReceiver<ToPeerMsg>,
    node_outlet: mpsc::UnboundedSender<ToNodeMsg>,
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
        let mut peer_outlet = peer_outlet.fuse();
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
                                node_outlet.clone(),
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

                from_node = peer_outlet.next() => {
                    match from_node {
                        Some(event) => handle_node_to_peer_event(
                            event,
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
