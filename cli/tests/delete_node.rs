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
use fluence::contract_func::contract::events::new_node;
use fluence::contract_func::contract::events::node_deleted;

#[cfg(test)]
fn delete_enqueued_node(wait_eth_sync: bool, wait_tx_include: bool) {
    let mut opts = TestOpts::default()
        .with_eth_sync(wait_eth_sync)
        .with_tx_include(wait_tx_include);

    let (tx, _) = opts.register_node(1, false).unwrap();

    let logs = opts.get_transaction_logs(&tx.unwrap(), new_node::parse_log);
    let log = logs.first().unwrap();
    let node_id = log.id;

    let tx = opts.delete_node(node_id).unwrap();
    let logs = opts.get_transaction_logs(&tx, node_deleted::parse_log);
    let log = logs.first().unwrap();
    assert_eq!(node_id, log.id);
}

#[cfg(test)]
fn deleted_deployed_node(wait_eth_sync: bool, wait_tx_include: bool) {
    let mut opts = TestOpts::default()
        .with_eth_sync(wait_eth_sync)
        .with_tx_include(wait_tx_include);

    let (tx, _) = opts.register_node(1, false).unwrap();
    let logs = opts.get_transaction_logs(&tx.unwrap(), new_node::parse_log);
    let log = logs.first().unwrap();
    let node_id = log.id;

    let tx = opts.publish_app(1, vec![node_id]).unwrap();
    let logs = opts.get_transaction_logs(&tx, app_deployed::parse_log);
    let app_id = logs.first().unwrap().app_id;

    let tx = opts.delete_node(node_id).unwrap();
    let logs = opts.get_transaction_logs(&tx, node_deleted::parse_log);
    let log = logs.first().unwrap();
    assert_eq!(node_id, log.id);

    let logs = opts.get_transaction_logs(&tx, app_deleted::parse_log);
    let log = logs.first().unwrap();
    assert_eq!(app_id, log.app_id);
}

// TODO: use macros to generate tests
#[test]
fn integration_delete_enqueued_node() {
    delete_enqueued_node(false, false)
}

#[test]
fn integration_delete_enqueued_node_wait_eth_sync() {
    delete_enqueued_node(true, false)
}

#[test]
fn integration_delete_enqueued_node_wait_tx_include() {
    delete_enqueued_node(false, true)
}

#[test]
fn integration_delete_enqueued_node_wait_eth_sync_and_tx_include() {
    delete_enqueued_node(true, true)
}

#[test]
fn integration_deleted_deployed_node() {
    deleted_deployed_node(false, false)
}

#[test]
fn integration_deleted_deployed_node_wait_eth_sync() {
    deleted_deployed_node(true, false)
}

#[test]
fn integration_deleted_deployed_node_wait_tx_include() {
    deleted_deployed_node(false, true)
}

#[test]
fn integration_deleted_deployed_node_wait_eth_sync_and_tx_include() {
    deleted_deployed_node(true, true)
}
