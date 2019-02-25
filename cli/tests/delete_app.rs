/*
 * Copyright 2019 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

pub mod utils;

use crate::utils::*;
use fluence::contract_func::contract::events::app_deleted;
use fluence::contract_func::contract::events::app_deployed;
use fluence::contract_func::contract::events::app_dequeued;
use fluence::contract_func::contract::events::app_enqueued;

#[cfg(test)]
fn delete_app(wait_eth_sync: bool, wait_tx_include: bool) {
    let mut opts = TestOpts::default()
        .with_eth_sync(wait_eth_sync)
        .with_tx_include(wait_tx_include);

    let count = 7;
    for _ in 0..count {
        opts.register_random_node(1, false).unwrap();
    }

    let tx = opts.publish_app(count, vec![]).unwrap();

    // print out status in case test is failed
    let status = opts.get_status();
    if let Ok(status) = status {
        let json = serde_json::to_string_pretty(&status).unwrap();

        println!("{}", json);
    }

    let logs = opts.get_transaction_logs(&tx, app_deployed::parse_log);
    let log = logs.first().unwrap();
    let app_id = log.app_id;

    let tx = opts.delete_app(app_id.into(), true).unwrap();

    let logs = opts.get_transaction_logs(&tx, app_deleted::parse_log);
    let log = logs.first().unwrap();

    assert_eq!(log.app_id, app_id);
}

#[cfg(test)]
fn dequeue_app(wait_eth_sync: bool, wait_tx_include: bool) {
    let opts = TestOpts::default()
        .with_eth_sync(wait_eth_sync)
        .with_tx_include(wait_tx_include);

    let tx = opts.publish_app(50, vec![]).unwrap();

    let logs = opts.get_transaction_logs(&tx, app_enqueued::parse_log);
    let log = logs.first().unwrap();
    let app_id = log.app_id;

    let tx = opts.delete_app(app_id.into(), false).unwrap();

    let logs = opts.get_transaction_logs(&tx, app_dequeued::parse_log);
    let log = logs.first().unwrap();

    assert_eq!(log.app_id, app_id);
}

// TODO: use macros to generate tests
#[test]
fn integration_delete_app() {
    delete_app(false, false)
}

#[test]
fn integration_delete_app_wait_eth_sync() {
    delete_app(true, false)
}

#[test]
fn integration_delete_app_wait_tx_include() {
    delete_app(false, true)
}

#[test]
fn integration_delete_app_wait_eth_sync_and_tx_include() {
    delete_app(true, true)
}

#[test]
fn integration_dequeue_app() {
    dequeue_app(false, false)
}

#[test]
fn integration_dequeue_app_wait_eth_sync() {
    dequeue_app(true, false)
}

#[test]
fn integration_dequeue_app_wait_tx_include() {
    dequeue_app(false, true)
}

#[test]
fn integration_dequeue_app_wait_eth_sync_and_tx_include() {
    dequeue_app(true, true)
}
