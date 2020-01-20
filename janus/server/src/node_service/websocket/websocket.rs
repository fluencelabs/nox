use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    sync::{Arc, Mutex},
};

use std::str::FromStr;
use std::task::{Context, Poll};


use crate::node_service::websocket::messages::WebsocketMessage;
use futures::channel::{mpsc, oneshot};

use serde_json::{Result as SerdeResult, Value};
use serde::{Serialize, Deserialize};

use tungstenite::handshake::server::{Request, ErrorResponse};
use tungstenite::http::StatusCode;

use libp2p::PeerId;

use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future, pin_mut,
    stream::TryStreamExt,
    StreamExt,
};

use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use tungstenite::protocol::Message;
use crate::peer_service::notifications::{InPeerNotification, OutPeerNotification};
use crate::config::WebsocketConfig;
use crate::node_service::websocket::messages::WebsocketMessage::RelayOut;

type Tx = UnboundedSender<Message>;
type ConnectionMap = Arc<Mutex<HashMap<PeerId, Tx>>>;
type PeerIdOption = Arc<Mutex<Option<PeerId>>>;

async fn handle_connection(peer_map: ConnectionMap, raw_stream: TcpStream, peer_id_o: PeerIdOption,
                           peer_channel_in: mpsc::UnboundedSender<OutPeerNotification>) {
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

        peer_id_o.lock().unwrap().replace(key.clone());

        println!("The request's headers are:");
        for &(ref header, _) in req.headers.iter() {
            println!("* {}", header);
        }

        Ok(None)
    };

    let ws_stream = async_tungstenite::accept_hdr_async(raw_stream, callback)
        .await
        .expect("Error during the websocket handshake occurred");

    let peer_id_new = peer_id_o.lock().unwrap().as_ref().unwrap().clone();

    println!("WebSocket connection established: {}", peer_id_new);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(peer_id_new.clone(), tx);

    let (outgoing, incoming) = ws_stream.split();

    peer_channel_in.unbounded_send(OutPeerNotification::PeerConnected {peer_id: peer_id_new.clone()}).unwrap();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        let text = msg.to_text().unwrap();

        println!(
            "Received a message from {}: {}",
            peer_id_new,
            text
        );
        let peers = peer_map.lock().unwrap();

        let wmsg: WebsocketMessage = match serde_json::from_str(text) {
            Err(_) => {
                let err_msg = Message::Text("Cannot parse message.".to_string());
                peers.get(&peer_id_new).unwrap().unbounded_send(err_msg);
                return future::ok(());
            }
            Ok(v) => v,
        };
        println!("wmsg: {:?}", wmsg);

        match wmsg {
            WebsocketMessage::RelayIn{ dst, data } => {
                let dst_peer_id = PeerId::from_str(dst.as_str()).unwrap();
                let msg = OutPeerNotification::Relay {src_id: peer_id_new.clone(), dst_id: dst_peer_id, data: data.into_bytes()};
                peer_channel_in.unbounded_send(msg).unwrap();
            },
            WebsocketMessage::RelayOut{ src, data } => {

            },
            WebsocketMessage::GetNetworkState => {

            },
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    peer_channel_in.unbounded_send(OutPeerNotification::PeerDisconnected {peer_id: peer_id_new.clone()}).unwrap();

    println!("{} disconnected", peer_id_new);
    peer_map.lock().unwrap().remove(&peer_id_new);
}

fn handle_incoming(mut peer_channel_out: mpsc::UnboundedReceiver<InPeerNotification>, peer_map: ConnectionMap) -> impl futures::Future<Output = Result<(), ()>> {
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
                                let msg = RelayOut {
                                    src: src_id.to_base58(),
                                    data: String::from_utf8(data.clone()).unwrap()
                                };
                                let msg = serde_json::to_string(&msg).unwrap();
                                let msg = tungstenite::protocol::Message::Text(msg);
                                recp.unbounded_send(msg).unwrap();
                            }
                        },

                    InPeerNotification::NetworkState { dst_id, state } => {
                        let peers = peer_map.lock().unwrap();
                        let broadcast_recipients = peers
                            .iter()
                            .find(|(peer_addr, _)| peer_addr == &&dst_id)
                            .map(|(_, ws_sink)| ws_sink);

                        for recp in broadcast_recipients {
                            let msg = RelayOut {
                                src: dst_id.to_base58(),
                                data: "data.to_hex()".to_string()
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

pub async fn run_websocket(config: WebsocketConfig,
                       mut peer_channel_out: mpsc::UnboundedReceiver<InPeerNotification>,
                       peer_channel_in: mpsc::UnboundedSender<OutPeerNotification>) -> Result<(), IoError> {
    let addr = format!("{}:{}", config.listen_ip, config.listen_port).to_string();

    let peer_map = ConnectionMap::new(Mutex::new(HashMap::new()));
    let peer_id = Arc::new(Mutex::new(None));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");

    println!("binding address");

    let f = handle_incoming(peer_channel_out, peer_map.clone());

    task::spawn(f);

    println!("handling incoming messages");

    while let Ok((stream, _addr)) = listener.accept().await {
        task::spawn(handle_connection(peer_map.clone(), stream, peer_id.clone(), peer_channel_in.clone()));
    }

    println!("accepting connections");

    Ok(())
}
