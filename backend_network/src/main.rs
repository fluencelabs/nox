#![recursion_limit = "256"]

#[macro_use]
extern crate lazy_static;

use futures::{future, prelude::*};
use libp2p::{
    build_development_transport,
    floodsub::{Floodsub, FloodsubEvent},
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    tokio_codec::{FramedRead, LinesCodec},
    tokio_io::{AsyncRead, AsyncWrite},
    NetworkBehaviour, PeerId, Swarm,
};
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    static ref MESSAGE_MAP: Mutex<HashMap<String, String>> =
        Mutex::new(HashMap::<String, String>::new());
}

#[derive(NetworkBehaviour)]
struct FluenceBehaviour<TSubstream: AsyncRead + AsyncWrite> {
    //peers: Vec<PeerId>,
    floodsub: Floodsub<TSubstream>,
    mdns: Mdns<TSubstream>,
}

impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<MdnsEvent>
    for FluenceBehaviour<TSubstream>
{
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer_id, multiaddr) in list {
                    println!(
                        "new peer discovered: per_id - {:?}, multiaddr - {:?}",
                        peer_id, multiaddr
                    );
                    //self.peers.push_back((peer_id, multiaddr));
                    self.floodsub.add_node_to_partial_view(peer_id);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer_id, _) in list {
                    if !self.mdns.has_node(&peer_id) {
                        self.floodsub.remove_node_from_partial_view(&peer_id);
                    }
                }
            }
        }
    }
}

impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<FloodsubEvent>
    for FluenceBehaviour<TSubstream>
{
    // Called when `floodsub` produces an event.
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            let message_data = unsafe { String::from_utf8_unchecked(message.data) };
            println!(
                "floodsub read message {} from {}",
                message_data, message.source
            );

            let lines = message_data.split("\n").collect::<Vec<_>>();

            let mut map = MESSAGE_MAP.lock().unwrap();
            map.insert(lines[0].to_string(), lines[1].to_string());
        }
    }
}

fn main() {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let transport = build_development_transport(local_key);
    let mut behaviour = FluenceBehaviour {
        //peers: Vec::new(),
        floodsub: Floodsub::new(local_peer_id.clone()),
        mdns: Mdns::new().expect("failed to create the mdns service"),
    };

    let floodsub_topic = libp2p::floodsub::TopicBuilder::new("fluence_peer").build();
    behaviour.floodsub.subscribe(floodsub_topic.clone());

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

    // Read full lines from stdin
    let stdin = tokio_stdin_stdout::stdin(0);
    let mut framed_stdin = FramedRead::new(stdin, LinesCodec::new());

    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

    let mut listening = false;
    tokio::run(future::poll_fn(move || -> Result<_, ()> {
        loop {
            match framed_stdin.poll().expect("Error while polling stdin") {
                Async::Ready(Some(line)) => {
                    let lines = line.split(" ").collect::<Vec<_>>();
                    match lines[0] {
                        "get" => {
                            if lines.len() == 2 {
                                let map = MESSAGE_MAP.lock().unwrap();
                                match map.get(lines[1]) {
                                    Some(v) => println!("value for key {} is {}", lines[1], v),
                                    None => println!("nothing found for key {}", lines[1]),
                                }
                            } else {
                                println!("please specify exactly one key");
                            }
                        }
                        "put" => {
                            if lines.len() == 3 {
                                swarm.floodsub.publish(
                                    &floodsub_topic,
                                    (lines[1].to_string() + "\n" + lines[2]).as_bytes(),
                                );

                                let mut map = MESSAGE_MAP.lock().unwrap();
                                map.insert(lines[1].to_string(), lines[2].to_string());
                            }
                        }
                        _ => {
                            println!("expect only put or get");
                        }
                    }
                }
                Async::Ready(None) => panic!("Stdin closed"),
                Async::NotReady => break,
            };
        }

        loop {
            match swarm.poll().expect("Error while polling swarm") {
                Async::Ready(Some(_)) => {}
                Async::Ready(None) | Async::NotReady => {
                    if !listening {
                        if let Some(a) = Swarm::listeners(&swarm).next() {
                            println!("Listening on {:?}", a);
                            listening = true;
                        }
                    }

                    return Ok(Async::NotReady);
                }
            }
        }

        //Ok(Async::NotReady)
    }));
}
