mod utils;

use crate::utils::*;
use fluence::contract_func::contract::events::app_deleted;
use fluence::contract_func::contract::events::app_deployed;
use fluence::contract_func::contract::events::app_dequeued;
use fluence::contract_func::contract::events::app_enqueued;

#[test]
fn integration_delete_app() {
    let mut opts = TestOpts::default();

    let count = 5;
    for _ in 0..count {
        opts.register_node(1, false).unwrap();
    }

    let tx = opts.publish_app(count, vec![]).unwrap();

    let logs = opts.get_transaction_logs(tx, app_deployed::parse_log);
    let log = logs.first().unwrap();
    let app_id = log.app_id;

    let tx = opts.delete_app(app_id, true).unwrap();

    let logs = opts.get_transaction_logs(tx, app_deleted::parse_log);
    let log = logs.first().unwrap();

    assert_eq!(log.app_id, app_id);
}

#[test]
fn integration_dequeue_app() {
    let opts = TestOpts::default();

    let tx = opts.publish_app(50, vec![]).unwrap();

    let logs = opts.get_transaction_logs(tx, app_enqueued::parse_log);
    let log = logs.first().unwrap();
    let app_id = log.app_id;

    let tx = opts.delete_app(app_id, false).unwrap();

    let logs = opts.get_transaction_logs(tx, app_dequeued::parse_log);
    let log = logs.first().unwrap();

    assert_eq!(log.app_id, app_id);
}
