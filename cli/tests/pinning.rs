mod utils;

use crate::utils::*;
use fluence::contract_status::status::{get_status, Status};
use fluence::register::Register;
use web3::types::H256;
use fluence::contract_func::contract::events::app_deployed;

#[test]
fn integration_publish_pinned() -> () {
    let mut opts = TestOpts::default();

    let count = 5;
    let nodes: Result<Vec<Register>> = (0..count).map(|_| opts.register_node(1, true)).collect();
    let nodes = nodes.unwrap();
    let node_ids: Vec<H256> = nodes.iter().map(|n| *n.tendermint_key()).collect();

    opts.publish_app(5, node_ids).unwrap();

    let logs = opts.get_logs(app_deployed::filter(), app_deployed::parse_log);
    let log = logs.first().unwrap();

    assert_eq!(log.node_i_ds.len(), count);

    let status: Status = get_status(*opts.contract_address(), opts.eth_url()).unwrap();

    let target = status
        .apps()
        .first()
        .unwrap();
    let pins = target.pin_to_nodes().as_ref().unwrap();
    assert_eq!(pins.len(), count);
}
