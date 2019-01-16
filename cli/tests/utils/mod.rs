use fluence::credentials::Credentials;
use fluence::publisher::Publisher;
use fluence::register::Register;

use std::error::Error;
use std::result::Result as StdResult;

use rand::Rng;
use web3::types::{Address, H256};

use derive_getters::Getters;
use ethabi::TopicFilter;
use web3::transports::Http;
use futures::future::Future;
use ethabi::RawLog;
use web3::types::FilterBuilder;

pub type Result<T> = StdResult<T, Box<Error>>;

#[derive(Debug, Getters)]
pub struct TestOpts {
    contract_address: Address,
    account: Address,
    start_port: u16,
    credentials: Credentials,
    eth_url: String,
    last_used_port: Option<u16>,
    gas: u32,
    code_bytes: Vec<u8>,
    swarm_url: String,
}

impl TestOpts {
    pub fn default() -> TestOpts {
        TestOpts {
            contract_address: "9995882876ae612bfd829498ccd73dd962ec950a".parse().unwrap(),
            account: "4180fc65d613ba7e1a385181a219f1dbfe7bf11d".parse().unwrap(),
            start_port: 25000,
            credentials: Credentials::No,
            eth_url: String::from("http://localhost:8545/"),
            last_used_port: None,
            gas: 1_000_000,
            code_bytes: vec![1, 2, 3],
            swarm_url: String::from("http://localhost:8500"),
        }
    }

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
        TestOpts {
            contract_address,
            account,
            start_port,
            credentials,
            eth_url,
            last_used_port: None,
            gas,
            code_bytes,
            swarm_url,
        }
    }

    pub fn register_node(&mut self, ports: u16, private: bool) -> Result<Register> {
        let mut rng = rand::thread_rng();
        let rnd_num: u64 = rng.gen();
        let tendermint_key: H256 = H256::from(rnd_num);

        let start_port = self.last_used_port.unwrap_or(self.start_port);
        let end_port = start_port + ports;

        self.last_used_port = Some(end_port + 1);

        let reg = Register::new(
            "127.0.0.1".parse().unwrap(),
            tendermint_key,
            start_port,
            end_port,
            self.contract_address,
            self.account,
            self.eth_url.clone(),
            self.credentials.clone(),
            false,
            self.gas,
            private,
        )
        .unwrap();

        reg.register(false)?;

        Ok(reg)
    }

    pub fn publish_app(&self, cluster_size: u8, pin_to: Vec<H256>) -> Result<Publisher> {
        let publish = Publisher::new(
            self.code_bytes.clone(),
            self.contract_address,
            self.account,
            self.swarm_url.clone(),
            self.eth_url.clone(),
            self.credentials.clone(),
            cluster_size,
            self.gas,
            pin_to,
        );

        publish.publish(false)?;

        Ok(publish)
    }

    // retrieves all events matching `filter`, parsing them through `parse_log`
    // Example usage:
    // use fluence::contract_func::contract::events::app_deployed;
    // get_logs(app_deployed::filter(), app_deployed::parse_log);
    pub fn get_logs<T, F>(&self, filter: TopicFilter, parse_log: F) -> Vec<T>
    where
        F: Fn(RawLog) -> ethabi::Result<T>
    {
        let (_eloop, transport) = Http::new(&self.eth_url).unwrap();
        let web3 = web3::Web3::new(transport);
        let filter = FilterBuilder::default().address(vec![self.contract_address]).topic_filter(filter).build();
        let filter = web3.eth_filter().create_logs_filter(filter).wait().unwrap();
        let logs = filter.logs().wait().unwrap();
        let logs: Vec<T> = logs.into_iter().map(|l| {
            let raw = RawLog::from((l.topics, l.data.0));
            parse_log(raw).unwrap()
        }).collect();

        logs
    }
}
