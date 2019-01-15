extern crate fluence;

mod utils;

use crate::utils::*;

#[test]
fn publish_pinned() -> () {
    let mut opts = TestOpts::default();

    let nodes = (0..5).iter().map(|| opts.register_node(1, true)).collect();
    let node_ids = nodes.iter().map(|n| n.tendermint_key).collect();
}
