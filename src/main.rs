use std::env;

use dialer::dial;
use server::serve;

mod dialer;
mod server;

// DONE: connect with js
// DONE: secio
// TODO: use webrtcStar? https://github.com/libp2p/js-libp2p/tree/master/examples/libp2p-in-the-browser/1/src
// TODO: refactor out common code (I tried and haven't succeeded: ExpandedSwarm type is a complex beast)
// TODO: make compatible with JS pubsub strictSigning
// TODO: merge circuit relay?

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "dial" {
        dial(args[2].as_str())
    } else {
        serve(30000)
    }
}
