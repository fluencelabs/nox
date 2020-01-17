use std::env;

use dialer::dial;
use server::serve;

mod dialer;
mod server;
mod transport;
mod behaviour;

// DONE: connect with js
// DONE: secio
// DONE: ping; // TODO: make sure every party sends both Pings and Pongs
// TODO (NOT NOW): use webrtcStar? https://github.com/libp2p/js-libp2p/tree/master/examples/libp2p-in-the-browser/1/src
// TODO (NOT NOW): use Go direct webrtc libp2p transport via C ABI?
// TODO: refactor out common code (I tried and haven't succeeded: ExpandedSwarm type is a complex beast)
// TODO: is it possible to be compatible with JS pubsub strictSigning?
// TODO: merge circuit relay?
// TODO: both rust & js should restore lost connection
// TODO: use websocket instead of tcp
// TODO: run JS in browser (websocket, not webrtc)
// TODO: several browsers connecting to several rust nodes
// TODO: implement our own circuit-relay

// TODO: debug rpc: give me peer list

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "dial" {
        dial(args[2].as_str())
    } else {
        serve(30000)
    }
}
