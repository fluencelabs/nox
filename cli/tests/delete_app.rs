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

    let logs = opts.get_transaction_logs(&tx, app_deployed::parse_log);
    let log = logs.first().unwrap();
    let app_id = log.app_id;

    let tx = opts.delete_app(app_id.into(), true).unwrap();

    let logs = opts.get_transaction_logs(&tx, app_deleted::parse_log);
    let log = logs.first().unwrap();

    assert_eq!(log.app_id, app_id);
}

#[test]
fn integration_dequeue_app() {
    let opts = TestOpts::default();

    let tx = opts.publish_app(50, vec![]).unwrap();

    let logs = opts.get_transaction_logs(&tx, app_enqueued::parse_log);
    let log = logs.first().unwrap();
    let app_id = log.app_id;

    let tx = opts.delete_app(app_id.into(), false).unwrap();

    let logs = opts.get_transaction_logs(&tx, app_dequeued::parse_log);
    let log = logs.first().unwrap();

    assert_eq!(log.app_id, app_id);
}
