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
use fluence::contract_func::contract::events::app_deployed;
use fluence::contract_status::status::{get_status, Status};
use fluence::register::Register;
use web3::types::H256;

#[test]
fn integration_publish_pinned() {
    let mut opts = TestOpts::default();

    let count = 5;
    let nodes: Result<Vec<(H256, Register)>> =
        (0..count).map(|_| opts.register_node(1, true)).collect();
    let nodes = nodes.unwrap();
    let node_ids: Vec<H256> = nodes.iter().map(|(_, n)| *n.tendermint_key()).collect();

    let tx = opts.publish_app(count, node_ids).unwrap();

    let logs = opts.get_transaction_logs(&tx, app_deployed::parse_log);
    let log = logs.first().unwrap();
    let app_id: u64 = log.app_id.into();

    assert_eq!(log.node_i_ds.len(), count as usize);

    let status: Status =
        get_status(opts.eth().contract_address, opts.eth().eth_url.as_str()).unwrap();

    let target = status
        .apps()
        .into_iter()
        .find(|a| *a.app_id() == app_id)
        .unwrap();
    let pins = target.pin_to_nodes().as_ref().unwrap();
    assert_eq!(pins.len(), count as usize);
}
