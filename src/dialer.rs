use std::time::Duration;

use futures::{future, prelude::*};
use libp2p::{
    identity,
    ping::{Ping, PingConfig},
    PeerId, Swarm,
};

pub fn dial(addr: &str) {
    // Create a random PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    println!("peer id: {}", local_peer_id);

    // Set up a an encrypted DNS-enabled TCP Transport over the Mplex and Yamux protocols
    let transport = libp2p::build_development_transport(local_key);

    let behaviour = Ping::new(
        PingConfig::new()
            .with_keep_alive(true)
            .with_interval(Duration::from_secs(1)),
    );

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id);

    let addr = addr.parse().unwrap();

    Swarm::dial_addr(&mut swarm, addr).expect("error dialing addr");

    tokio::run(future::poll_fn(move || -> Result<_, ()> {
        loop {
            match swarm.poll().expect("Error while polling swarm") {
                Async::Ready(Some(e)) => println!("sent {:?} to {:?}", e.result, e.peer),
                Async::Ready(None) | Async::NotReady => {
                    return Ok(Async::NotReady);
                }
            }
        }
    }));
}
