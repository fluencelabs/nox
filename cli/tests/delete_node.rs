mod utils;

use crate::utils::*;
use fluence::contract_func::contract::events::new_node;
use fluence::contract_func::contract::events::node_deleted;

#[test]
fn integration_delete_enqueued_node() {
    let mut opts = TestOpts::default();

    let (tx, _) = opts.register_node(1, false).unwrap();

    let logs = opts.get_transaction_logs(tx, new_node::parse_log);
    let log = logs.first().unwrap();
    let node_id = log.id;

    let tx = opts.delete_node(node_id).unwrap();
    let logs = opts.get_transaction_logs(tx, node_deleted::parse_log);
    let log = logs.first().unwrap();
    assert_eq!(node_id, log.id)
}