use std::time::Duration;

use futures::{future, prelude::*};
use libp2p::{
    floodsub, identity,
    ping::{Ping, PingConfig},
    PeerId, Swarm,
};

use crate::server::Network;
use crate::transport;
use libp2p_identify::Identify;
use tokio::codec::{FramedRead, LinesCodec};

pub fn dial(addr: &str) {
    // Create a random PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    println!("peer id: {}", local_peer_id);

    // mplex + secio
    let transport = transport::build_mplex(local_key.clone());

    // Create a Floodsub topic
    let floodsub_topic = floodsub::TopicBuilder::new("chat").build();
    println!("floodsub topic is {:?}", floodsub_topic);

    let mut swarm = {
        let mut behaviour = Network {
            floodsub: floodsub::Floodsub::new(local_peer_id.clone()),
            identify: Identify::new("1.0.0".into(), "1.0.0".into(), local_key.public()),
            ping: Ping::new(PingConfig::with_keep_alive(PingConfig::new(), true)),
        };

        let result = behaviour.floodsub.subscribe(floodsub_topic.clone());
        println!("floodsub subscribe {}", result);
        Swarm::new(transport, behaviour, local_peer_id.clone())
    };

    let addr = addr.parse().unwrap();

    Swarm::dial_addr(&mut swarm, addr).expect("error dialing addr");

    let stdin = tokio_stdin_stdout::stdin(0);
    let mut framed_stdin = FramedRead::new(stdin, LinesCodec::new());

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

        loop {
            match swarm.poll().expect("Error while polling swarm") {
                Async::Ready(Some(e)) => println!("poll event {:?}", e),
                Async::Ready(None) | Async::NotReady => {
                    return Ok(Async::NotReady);
                }
            }
        }
    }));
}
