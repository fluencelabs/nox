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

mod utils;

use crate::utils::*;
use fluence::contract_func::contract::events::app_deleted;
use fluence::contract_func::contract::events::app_deployed;
use fluence::contract_func::contract::events::new_node;
use fluence::contract_func::contract::events::node_deleted;

#[test]
fn integration_delete_enqueued_node() {
    let mut opts = TestOpts::default();

    let (tx, _) = opts.register_node(1, false).unwrap();

    let logs = opts.get_transaction_logs(&tx, new_node::parse_log);
    let log = logs.first().unwrap();
    let node_id = log.id;

    let tx = opts.delete_node(node_id).unwrap();
    let logs = opts.get_transaction_logs(&tx, node_deleted::parse_log);
    let log = logs.first().unwrap();
    assert_eq!(node_id, log.id);
}

#[test]
fn integration_deleted_deployed_node() {
    let mut opts = TestOpts::default();

    let (tx, _) = opts.register_node(1, false).unwrap();
    let logs = opts.get_transaction_logs(&tx, new_node::parse_log);
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
