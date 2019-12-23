use futures::{future, prelude::*};
use libp2p::{
    identity,
    ping::{Ping, PingConfig},
    PeerId, Swarm, Transport,
};
use libp2p_core::muxing::StreamMuxer;
use libp2p_core::muxing::SubstreamRef;
use libp2p_ping::handler::{PingFailure, PingHandler};
use libp2p_ping::PingSuccess;
use libp2p_swarm::{ExpandedSwarm, NetworkBehaviour};
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use void::Void;

// TODO: connect with js
// TODO: secio
// TODO: what is webrtcStar? https://github.com/libp2p/js-libp2p/tree/master/examples/libp2p-in-the-browser/1/src
// TODO: refactor out common code (I tried and haven't succeeded: ExpandedSwarm type is a nightmare)

fn serve(port: i32) {
    // Create a random PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    println!("peer id: {}", local_peer_id);

    // Set up a an encrypted DNS-enabled TCP Transport over the Mplex and Yamux protocols
    let transport = libp2p::build_development_transport(local_key);

    let behaviour = Ping::new(PingConfig::new().with_keep_alive(true));

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id.clone());

    // Tell the swarm to listen on all interfaces and a random, OS-assigned port.
    let addr = format!("/ip4/127.0.0.1/tcp/{}", port).parse().unwrap();
    Swarm::listen_on(&mut swarm, addr).unwrap();

    // Use tokio to drive the `Swarm`.
    let mut listening = false;
    tokio::run(future::poll_fn(move || -> Result<_, ()> {
        loop {
            match swarm.poll().expect("Error while polling swarm") {
                Async::Ready(Some(e)) => println!("sent {:?} to {:?}", e.result, e.peer),
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

fn dial(addr: &str) {
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

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "dial" {
        dial(args[2].as_str())
    } else {
        serve(30000)
    }
}
