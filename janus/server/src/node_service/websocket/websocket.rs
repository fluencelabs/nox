use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use std::str::FromStr;
use std::task::{Context, Poll};


use crate::node_service::websocket::messages::WebsocketMessage;
use futures::channel::{mpsc, oneshot};

use tungstenite::handshake::server::{Request, ErrorResponse};
use tungstenite::http::StatusCode;

use libp2p::PeerId;

use futures::{channel::mpsc::{unbounded, UnboundedSender}, future, pin_mut, stream::TryStreamExt, StreamExt};

use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use tungstenite::protocol::Message;
use crate::peer_service::notifications::{InPeerNotification, OutPeerNotification};
use crate::config::WebsocketConfig;

type Tx = UnboundedSender<Message>;
type ConnectionMap = Arc<Mutex<HashMap<PeerId, Tx>>>;

async fn handle_connection(peer_map: ConnectionMap, raw_stream: TcpStream,
                           peer_channel_in: mpsc::UnboundedSender<OutPeerNotification>) {

    let (peer_id_sender, peer_id_receiver) = oneshot::channel();

    let callback = |req: &Request| {
        println!("Received a new ws handshake");
        println!("The request's path is: {}", req.path);

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

        //todo
        let key = req.path.split_at(index + 4).1;

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
        println!("key: {}", key);

        peer_id_sender.send(key.clone()).unwrap();

        Ok(None)
    };

    let ws_stream = async_tungstenite::accept_hdr_async(raw_stream, callback)
        .await
        .expect("Error during the websocket handshake occurred");

    let peer_id = peer_id_receiver.await.unwrap();

    println!("WebSocket connection established: {}", peer_id);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(peer_id.clone(), tx);

    let (outgoing, incoming) = ws_stream.split();

    peer_channel_in.unbounded_send(OutPeerNotification::PeerConnected {peer_id: peer_id.clone()}).unwrap();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        return handle_message(msg, peer_id.clone(), peer_channel_in.clone())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    peer_channel_in.unbounded_send(OutPeerNotification::PeerDisconnected {peer_id: peer_id.clone()}).unwrap();

    println!("{} disconnected", peer_id);
    peer_map.lock().unwrap().remove(&peer_id);
}

fn handle_message(msg: tungstenite::Message, self_peer_id: PeerId, peer_channel_in: mpsc::UnboundedSender<OutPeerNotification>) -> impl futures::Future<Output = Result<(), tungstenite::error::Error>> {
    let text = match msg.to_text() {
        Ok(r) => r,
        Err(e) => return future::err(e)
    };

    println!(
        "Received a message from {}: {}",
        self_peer_id,
        text
    );

    let wmsg: WebsocketMessage = match serde_json::from_str(text) {
        Err(_) => {
            println!("Cannot parse message: {}", text);
            return future::ok(());
        }
        Ok(v) => v,
    };

    match wmsg {
        WebsocketMessage::Relay{ peer_id, data } => {
            let dst_peer_id = PeerId::from_str(peer_id.as_str()).unwrap();
            let msg = OutPeerNotification::Relay {src_id: self_peer_id.clone(), dst_id: dst_peer_id, data: data.into_bytes()};
            peer_channel_in.unbounded_send(msg).unwrap();
        },
        WebsocketMessage::GetNetworkState => {
            let msg = OutPeerNotification::GetNetworkState {src_id: self_peer_id.clone()};
            peer_channel_in.unbounded_send(msg).unwrap();
        },
        _ => {}
    }

    future::ok(())
}

fn handle_incoming(mut peer_channel_out: mpsc::UnboundedReceiver<InPeerNotification>, peer_map: ConnectionMap) -> impl futures::Future<Output = Result<(), tungstenite::error::Error>> {
    futures::future::poll_fn(move |cx: &mut Context| {
        loop {
            match peer_channel_out.poll_next_unpin(cx) {
                Poll::Ready(Some(e)) => match e {
                    InPeerNotification::Relay {
                        src_id,
                        dst_id,
                        data,
                    } =>
                        {
                            let peers = peer_map.lock().unwrap();
                            let broadcast_recipients = peers
                                .iter()
                                .find(|(peer_addr, _)| peer_addr == &&dst_id)
                                .map(|(_, ws_sink)| ws_sink);

                            for recp in broadcast_recipients {
                                let msg = WebsocketMessage::Relay {
                                    peer_id: src_id.to_base58(),
                                    data: String::from_utf8(data.clone()).unwrap()
                                };
                                let msg = serde_json::to_string(&msg).unwrap();
                                let msg = tungstenite::protocol::Message::Text(msg);
                                recp.unbounded_send(msg).unwrap();
                            }
                        },

                    InPeerNotification::NetworkState { dst_id, state } => {
                        println!("network state: {:?}", state);
                        let peers = peer_map.lock().unwrap();
                        let broadcast_recipients = peers
                            .iter()
                            .find(|(peer_addr, _)| peer_addr == &&dst_id)
                            .map(|(_, ws_sink)| ws_sink);

                        for recp in broadcast_recipients {
                            let msg = WebsocketMessage::NetworkState {
                                peers: state.iter().map(|p| p.to_base58()).collect()
                            };
                            let msg = serde_json::to_string(&msg).unwrap();
                            let msg = tungstenite::protocol::Message::Text(msg);
                            recp.unbounded_send(msg).unwrap();
                        }
                    },
                },
                Poll::Pending => break,
                Poll::Ready(None) => {
                    // TODO: propagate error
                    break;
                }
            }
        }

        Poll::Pending
    })
}

fn handle_connections(listener: TcpListener, peer_map: ConnectionMap, peer_channel_in: mpsc::UnboundedSender<OutPeerNotification>) -> impl futures::Future<Output = Result<(), tungstenite::error::Error>> {
    futures::future::poll_fn(move |cx: &mut Context| {
        loop {
            match listener.incoming().poll_next_unpin(cx) {
                Poll::Ready(Some(r)) => {
                    match r {
                        Ok(stream) => {
                            task::spawn(handle_connection(peer_map.clone(), stream, peer_channel_in.clone()));
                        },
                        Err(e) => println!("Error {:?}", e)
                    }

                },
                Poll::Pending => break,
                Poll::Ready(None) => {
                    // TODO: propagate error
                    break;
                }
            }
        }

        Poll::Pending
    })
}

pub async fn start_peer_service(config: WebsocketConfig,
                                peer_channel_out: mpsc::UnboundedReceiver<InPeerNotification>,
                                peer_channel_in: mpsc::UnboundedSender<OutPeerNotification>) -> oneshot::Sender<()> {
    let addr = format!("{}:{}", config.listen_ip, config.listen_port).to_string();

    let (exit_sender, exit_receiver) = oneshot::channel();

    let peer_map = ConnectionMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");

    println!("binding address");

    task::spawn(handle_incoming(peer_channel_out, peer_map.clone()));

    println!("handling incoming messages");

    task::spawn(future::select(handle_connections(listener, peer_map, peer_channel_in), exit_receiver));

    println!("accepting connections");

    exit_sender
}
