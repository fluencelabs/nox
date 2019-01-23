use clap::ArgMatches;
use clap::{App, Arg, SubCommand};
use web3::types::H256;

use crate::command::{EthereumArgs, parse_ethereum_args, with_ethereum_args, tendermint_key, base64_tendermint_key, parse_tendermint_key};
use crate::contract_func::contract::functions::delete_app;
use crate::contract_func::contract::functions::dequeue_app;
use crate::contract_func::ContractCaller;
use crate::utils;
use failure::Error;

use crate::contract_func::contract::functions::delete_node;


pub struct DeleteNode {
    tendermint_key: H256,
    eth: EthereumArgs,
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    let args = &[tendermint_key().index(1), base64_tendermint_key()];
    SubCommand::with_name("delete_node")
        .about("Delete node from smart-contract")
        .args(with_ethereum_args(args).as_slice())
}

pub fn parse(args: &ArgMatches) -> Result<DeleteNode, Error> {
    let tendermint_key = parse_tendermint_key(args)?;
    let eth = parse_ethereum_args(args)?;

    Ok(DeleteNode {
        tendermint_key,
        eth
    })
}

impl DeleteNode {
    pub fn new(tendermint_key: H256, eth: EthereumArgs) -> DeleteNode {
        DeleteNode {
            tendermint_key, eth
        }
    }

    pub fn delete_node(self, show_progress: bool) -> Result<H256, Error> {
        let delete_node_fn = || -> Result<H256, Error> {
            let (call_data, _) = delete_node::call(self.tendermint_key);
            let contract =
                ContractCaller::new(self.eth.contract_address, &self.eth.eth_url.as_str())?;

            contract.call_contract(
                self.eth.account,
                &self.eth.credentials,
                call_data,
                self.eth.gas,
            )
        };

        if show_progress {
            utils::with_progress(
                "Deleting node from smart contract...",
                "1/1",
                "Node deleted.",
                delete_node_fn,
            )
        } else {
            delete_node_fn()
        }
    }
}