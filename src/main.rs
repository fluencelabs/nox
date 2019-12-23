use futures::{future, prelude::*};
use libp2p::identity::PublicKey;
use libp2p::{
    identity,
    ping::{Ping, PingConfig},
    Multiaddr, PeerId, Swarm,
};
use std::env;
use std::time::Duration;

use base64;

// TODO: connect with js
// TODO: secio
// TODO: what is webrtcStar? https://github.com/libp2p/js-libp2p/tree/master/examples/libp2p-in-the-browser/1/src
// TODO: refactor out common code (I tried and haven't succeeded: ExpandedSwarm type is a nightmare)

fn serve(port: i32) {
    // Create a random PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    println!("peer id: {}", local_peer_id);

    match local_key.public() {
        PublicKey::Ed25519(key) => println!("Public Key: {}", base64::encode(&key.encode())),
        _ => println!("Key isn't ed25519!!!!!"),
    }

    // Set up a an encrypted DNS-enabled TCP Transport over the Mplex and Yamux protocols
    let transport = libp2p::build_development_transport(local_key);

    let behaviour = Ping::new(PingConfig::new().with_keep_alive(true));

    let mut swarm = Swarm::new(transport, behaviour, local_peer_id.clone());

    // Tell the swarm to listen on all interfaces and a random, OS-assigned port.
    let addr: Multiaddr = format!("/ip4/127.0.0.1/tcp/{}", port).parse().unwrap();
    Swarm::listen_on(&mut swarm, addr.clone()).unwrap();
    Swarm::add_external_address(&mut swarm, addr.clone());

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

                        let ea = Swarm::external_addresses(&swarm).next();
                        println!("External address: {:?}", ea);

                        println!("Local peer id {}", Swarm::local_peer_id(&swarm));
                    } else {
                        println!("something happened in swarm runloop");
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
