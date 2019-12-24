use base64;
use futures::{future, prelude::*};
use identity::ed25519;
use identity::Keypair;
use libp2p::{
    floodsub::{self, Floodsub, FloodsubEvent},
    identity,
    identity::PublicKey,
    ping::{Ping, PingConfig, PingEvent},
    swarm::NetworkBehaviourEventProcess,
    tokio_codec::{FramedRead, LinesCodec},
    tokio_io::{AsyncRead, AsyncWrite},
    Multiaddr, NetworkBehaviour, PeerId, Swarm,
};
use libp2p_identify::{Identify, IdentifyEvent};

const PRIVATE_KEY: &str =
    "/O5p1cDNIyEkG3VP+LqozM+gArhSXUdWkKz6O+C6Wtr+YihU3lNdGl2iuH37ky2zsjdv/NJDzs11C1Vj0kClzQ==";

#[derive(NetworkBehaviour)]
struct MyBehaviour<TSubstream: AsyncRead + AsyncWrite> {
    floodsub: Floodsub<TSubstream>,
    identify: Identify<TSubstream>,
    ping: Ping<TSubstream>,
}

impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<FloodsubEvent>
    for MyBehaviour<TSubstream>
{
    // Called when `floodsub` produces an event.
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            println!(
                "Received floodsub msg: '{:?}' from {:?}",
                String::from_utf8_lossy(&message.data),
                message.source
            );
        }
    }
}

impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<IdentifyEvent>
    for MyBehaviour<TSubstream>
{
    fn inject_event(&mut self, _event: IdentifyEvent) {
        //        println!("Received identify event {:?}", event);
    }
}

impl<TSubstream: AsyncRead + AsyncWrite> NetworkBehaviourEventProcess<PingEvent>
    for MyBehaviour<TSubstream>
{
    fn inject_event(&mut self, _event: PingEvent) {
        println!("Received {:?}", _event)
    }
}

pub fn serve(port: i32) {
    // Create a random PeerId
    let mut local_key = base64::decode(PRIVATE_KEY).unwrap();
    let local_key = local_key.as_mut_slice();
    let local_key = Keypair::Ed25519(ed25519::Keypair::decode(local_key).unwrap());
    let local_peer_id = PeerId::from(local_key.public());

    println!("peer id: {}", local_peer_id);

    match local_key.public() {
        PublicKey::Ed25519(key) => println!("Public Key: {}", base64::encode(&key.encode())),
        _ => println!("Key isn't ed25519!!!!!"),
    }

    match local_key.clone() {
        identity::Keypair::Ed25519(pair) => {
            println!("PrivateKey: {}", base64::encode(&pair.encode().to_vec()))
        }
        _ => println!("Key isn't ed25519!!!!!"),
    }

    // Set up a an encrypted DNS-enabled TCP Transport over the Mplex and Yamux protocols
    let transport = libp2p::build_development_transport(local_key.clone());

    // Create a Floodsub topic
    let floodsub_topic = floodsub::TopicBuilder::new("chat").build();
    println!("floodsub topic is {:?}", floodsub_topic);

    let mut swarm = {
        let mut behaviour = MyBehaviour {
            floodsub: Floodsub::new(local_peer_id.clone()),
            identify: Identify::new("1.0.0".into(), "1.0.0".into(), local_key.public()),
            ping: Ping::new(PingConfig::with_keep_alive(PingConfig::new(), true)),
        };

        behaviour.floodsub.subscribe(floodsub_topic.clone());
        Swarm::new(transport, behaviour, local_peer_id.clone())
    };

    // Tell the swarm to listen on all interfaces and a random, OS-assigned port.
    let addr: Multiaddr = format!("/ip4/127.0.0.1/tcp/{}", port).parse().unwrap();
    Swarm::listen_on(&mut swarm, addr.clone()).unwrap();

    let stdin = tokio_stdin_stdout::stdin(0);
    let mut framed_stdin = FramedRead::new(stdin, LinesCodec::new());

    let mut listening = false;

    // Use tokio to drive the `Swarm`.
    tokio::run(future::poll_fn(move || -> Result<_, ()> {
        // Read input from stdin
        loop {
            match framed_stdin.poll().expect("Error while polling stdin") {
                Async::Ready(Some(line)) => {
                    println!("sending floodsub msg");
                    swarm.floodsub.publish(&floodsub_topic, line.as_bytes())
                }
                Async::Ready(None) => panic!("Stdin closed"),
                Async::NotReady => break,
            };
        }

        // Some comments on poll may be relevant https://github.com/libp2p/rust-libp2p/issues/1058
        loop {
            match swarm.poll().expect("Error while polling swarm") {
                Async::Ready(Some(e)) => println!("Got {:?} ready", e),
                Async::Ready(None) | Async::NotReady => {
                    if !listening {
                        if let Some(a) = Swarm::listeners(&swarm).next() {
                            println!("Listening on {}/p2p/{}", a, local_peer_id);
                            listening = true;
                        }
                    }

                    return Ok(Async::NotReady);
                }
            }
        }
    }));
}
