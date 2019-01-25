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

use fluence::credentials::Credentials;
use fluence::publisher::Publisher;
use fluence::register::Register;

use failure::Error;
use std::result::Result as StdResult;

use rand::Rng;
use web3::types::{Address, H256};

use derive_getters::Getters;
use ethabi::RawLog;
use ethabi::TopicFilter;
use fluence::command::EthereumArgs;
use fluence::delete_app::DeleteApp;
use fluence::delete_node::DeleteNode;
use futures::future::Future;
use web3::transports::Http;
use web3::types::FilterBuilder;
use web3::types::H160;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, Getters)]
pub struct TestOpts {
    start_port: u16,
    last_used_port: Option<u16>,
    code_bytes: Vec<u8>,
    swarm_url: String,
    eth: EthereumArgs,
}

impl TestOpts {
    pub fn default() -> TestOpts {
        let eth = EthereumArgs {
            contract_address: "9995882876ae612bfd829498ccd73dd962ec950a".parse().unwrap(),
            account: "4180fc65d613ba7e1a385181a219f1dbfe7bf11d".parse().unwrap(),
            credentials: Credentials::No,
            eth_url: String::from("http://localhost:8545/"),
            gas: 1_000_000,
        };

        TestOpts {
            start_port: 25000,
            last_used_port: None,
            code_bytes: vec![1, 2, 3],
            swarm_url: String::from("http://localhost:8500"),
            eth,
        }
    }

    #[allow(dead_code)]
    pub fn new(
        contract_address: Address,
        account: Address,
        start_port: u16,
        credentials: Credentials,
        eth_url: String,
        gas: u32,
        code_bytes: Vec<u8>,
        swarm_url: String,
    ) -> TestOpts {
        let eth = EthereumArgs {
            contract_address,
            account,
            credentials,
            eth_url,
            gas,
        };

        TestOpts {
            start_port,
            last_used_port: None,
            code_bytes,
            swarm_url,
            eth,
        }
    }

    pub fn register_node(&mut self, ports: u16, private: bool) -> Result<(H256, Register)> {
        let mut rng = rand::thread_rng();
        let rnd_num: u64 = rng.gen();
        let tendermint_key: H256 = H256::from(rnd_num);
        let tendermint_node_id: H160 = H160::from(rnd_num);

        let start_port = self.last_used_port.unwrap_or(self.start_port);
        let end_port = start_port + ports;

        self.last_used_port = Some(end_port + 1);

        let reg = Register::new(
            "127.0.0.1".parse().unwrap(),
            tendermint_key,
            tendermint_node_id,
            start_port,
            end_port,
            false,
            private,
            self.eth.clone(),
        )
        .unwrap();

        let tx = reg.register(false)?;

        Ok((tx, reg))
    }

    pub fn publish_app(&self, cluster_size: u8, pin_to: Vec<H256>) -> Result<H256> {
        let publish = Publisher::new(
            self.code_bytes.clone(),
            self.swarm_url.clone(),
            cluster_size,
            pin_to,
            self.eth.clone(),
        );

        publish.publish(false)
    }

    // retrieves all events matching `filter`, parsing them through `parse_log`
    // Example usage:
    // use fluence::contract_func::contract::events::app_deployed;
    // get_logs(app_deployed::filter(), app_deployed::parse_log);
    #[allow(dead_code)]
    pub fn get_logs<T, F>(&self, filter: TopicFilter, parse_log: F) -> Vec<T>
    where
        F: Fn(RawLog) -> ethabi::Result<T>,
    {
        let (_eloop, transport) = Http::new(&self.eth.eth_url.as_str()).unwrap();
        let web3 = web3::Web3::new(transport);
        let filter = FilterBuilder::default()
            .address(vec![self.eth.contract_address])
            .topic_filter(filter)
            .build();
        let filter = web3.eth_filter().create_logs_filter(filter).wait().unwrap();
        let logs = filter.logs().wait().unwrap();
        let logs: Vec<T> = logs
            .into_iter()
            .map(|l| {
                let raw = RawLog::from((l.topics, l.data.0));
                parse_log(raw).unwrap()
            })
            .collect();

        logs
    }

    #[allow(dead_code)]
    pub fn delete_app(&self, app_id: H256, deployed: bool) -> Result<H256> {
        let delete = DeleteApp::new(app_id, deployed, self.eth.clone());

        delete.delete_app(false)
    }

    #[allow(dead_code)]
    pub fn delete_node(&self, node_id: H256) -> Result<H256> {
        let delete = DeleteNode::new(node_id, self.eth.clone());

        delete.delete_node(false)
    }
}
