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

use rand::Rng;

use crate::utils::*;
use fluence::register::Registered;
use web3::types::{H160, H256};

#[test]
fn integration_register_alredy_registered_node_with_check() {
    let mut opts = TestOpts::default();

    let mut rng = rand::thread_rng();
    let rnd_num: u64 = rng.gen();
    let tendermint_key: H256 = H256::from(rnd_num);
    let tendermint_node_id: H160 = H160::from(rnd_num);

    let (register_result1, _) = opts
        .register_node(1, false, tendermint_key, tendermint_node_id)
        .unwrap();
    assert_ne!(register_result1, Registered::AlreadyRegistered);

    let (register_result2, _) = opts
        .register_node(1, false, tendermint_key, tendermint_node_id)
        .unwrap();
    assert_eq!(register_result2, Registered::AlreadyRegistered)
}
