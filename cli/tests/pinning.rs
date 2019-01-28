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
use fluence::contract_func::contract::events::app_deployed;
use fluence::contract_status::status::{get_status, Status};
use fluence::register::Register;
use web3::types::H256;

#[cfg(test)]
fn publish_pinned(wait_eth_sync: bool, wait_tx_include: bool) {
    let mut opts = TestOpts::default()
        .with_eth_sync(wait_eth_sync)
        .with_tx_include(wait_tx_include);

    let count = 5;
    let nodes: Result<Vec<(H256, Register)>> =
        (0..count).map(|_| opts.register_node(1, true)).collect();
    let nodes = nodes.unwrap();
    let node_ids: Vec<H256> = nodes.iter().map(|(_, n)| *n.tendermint_key()).collect();

    let tx = opts.publish_app(count, node_ids).unwrap();

    let logs = opts.get_transaction_logs(&tx, app_deployed::parse_log);
    let log = logs.first().unwrap();
    let app_id = log.app_id;

    assert_eq!(log.node_i_ds.len(), count as usize);

    let status: Status =
        get_status(opts.eth().eth_url.as_str(), opts.eth().contract_address).unwrap();

    let target = status
        .apps()
        .into_iter()
        .find(|a| *a.app_id() == app_id)
        .unwrap();
    let pins = target.pin_to_nodes().as_ref().unwrap();
    assert_eq!(pins.len(), count as usize);
}

#[test]
fn integration_publish_pinned() {
    publish_pinned(false, false)
}

#[test]
fn integration_publish_pinned_wait_eth_sync() {
    publish_pinned(true, false)
}

#[test]
fn integration_publish_pinned_wait_tx_include() {
    publish_pinned(false, true)
}

#[test]
fn integration_publish_pinned_wait_eth_sync_and_tx_include() {
    publish_pinned(true, true)
}
