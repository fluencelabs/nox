mod utils;

use crate::utils::*;
use fluence::credentials::Credentials;
use fluence::contract_func::contract::events::app_deployed;
use fluence::contract_func::contract::events::app_enqueued;
use fluence::contract_func::contract::events::app_dequeued;
use fluence::contract_func::contract::events::app_deleted;
use fluence::contract_status::status::{get_status, Status};
use ethkey::Secret;

#[test]
fn integration_delete_app() {
    let mut opts = TestOpts::default();

//    let mut opts = TestOpts::new(
//        "731a10897d267e19B34503aD902d0A29173Ba4B1".parse().unwrap(),
//        "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap(),
//        26000,
//        Credentials::Secret("4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7".parse::<Secret>().unwrap()),
//        String::from("http://localhost:8545/"),
//        *default.gas(),
//        vec![1, 2, 3],
//        String::from("http://localhost:8500/"),
//    );

    let status: Status = get_status(*opts.contract_address(), opts.eth_url()).unwrap();
    let json = serde_json::to_string_pretty(&status).unwrap();
    println!("{}", json);

    let count = 5;
    for _ in 0..count {
        opts.register_node(1, false).unwrap();
    }

    opts.publish_app(count, vec![]).unwrap();

    let logs = opts.get_logs(app_deployed::filter(), app_deployed::parse_log);
    let log = logs.first().unwrap();
    let app_id = log.app_id;

    opts.delete_app(app_id, true);

    let logs = opts.get_logs(app_deleted::filter(), app_deleted::parse_log);
    let log = logs.first().unwrap();

    assert_eq!(log.app_id, app_id);
}

#[test]
fn integration_dequeue_app() {
    let opts = TestOpts::default();
//    let opts = TestOpts::new(
//        "731a10897d267e19B34503aD902d0A29173Ba4B1".parse().unwrap(),
//        "00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap(),
//        26000,
//        Credentials::Secret("4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7".parse::<Secret>().unwrap()),
//        String::from("http://localhost:8545/"),
//        *default.gas(),
//        vec![1, 2, 3],
//        String::from("http://localhost:8500/"),
//    );

    opts.publish_app(50, vec![]).unwrap();

    let status: Status = get_status(*opts.contract_address(), opts.eth_url()).unwrap();
    let json = serde_json::to_string_pretty(&status).unwrap();
    println!("{}", json);

    let logs = opts.get_logs(app_enqueued::filter(), app_enqueued::parse_log);
    let log = logs.first().unwrap();
    let app_id = log.app_id;

    opts.delete_app(app_id, false);

    let logs = opts.get_logs(app_dequeued::filter(), app_dequeued::parse_log);
    let log = logs.first().unwrap();

    assert_eq!(log.app_id, app_id);
}