/*
 * Copyright 2018 Fluence Labs Limited
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

mod app;
mod cluster;
mod node;
mod status;

use self::status::{get_status, Status};
use clap::{App, Arg, ArgMatches, SubCommand};
use std::boxed::Box;
use std::error::Error;
use web3::types::Address;

const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("status")
        .about("Get status of the smart contract")
        .args(&[
            Arg::with_name(CONTRACT_ADDRESS)
                .alias(CONTRACT_ADDRESS)
                .required(true)
                .takes_value(true)
                .help("fluence contract address"),
            Arg::with_name(ETH_URL)
                .alias(ETH_URL)
                .long("eth_url")
                .short("e")
                .required(false)
                .takes_value(true)
                .help("http address to ethereum node")
                .default_value("http://localhost:8545/"),
        ])
}

/// Gets status about Fluence contract from ethereum blockchain.
pub fn get_status_by_args(args: &ArgMatches) -> Result<Status, Box<Error>> {
    let contract_address: Address = args
        .value_of(CONTRACT_ADDRESS)
        .unwrap()
        .trim_start_matches("0x")
        .parse()?;
    let eth_url: &str = args.value_of(ETH_URL).unwrap();

    get_status(contract_address, eth_url)
}

#[cfg(test)]
mod tests {
    use super::get_status;
    use credentials::Credentials;
    use publisher::Publisher;
    use rand::prelude::*;
    use register::Register;
    use std::error::Error;
    use web3::types::*;

    const OWNER: &str = "4180FC65D613bA7E1a385181a219F1DBfE7Bf11d";
    const CONTRACT_ADDR: &str = "9995882876ae612bfd829498ccd73dd962ec950a";
    const ETH_URL: &str = "http://localhost:8545/";
    const SWARM_URL: &str = "http://localhost:8500";

    fn generate_publisher(bytes: Vec<u8>, cluster_size: &u8) -> Publisher {
        let contract_address: Address = CONTRACT_ADDR.parse().unwrap();

        let creds: Credentials = Credentials::No;

        Publisher::new(
            bytes,
            contract_address,
            OWNER.parse().unwrap(),
            String::from(SWARM_URL),
            String::from(ETH_URL),
            creds,
            cluster_size.to_owned(),
            1_000_000,
            vec![],
        )
    }

    fn generate_register(address: &str, start_port: u16, last_port: u16) -> Register {
        let contract_address: Address = CONTRACT_ADDR.parse().unwrap();

        let mut rng = rand::thread_rng();
        let rnd_num: u64 = rng.gen();

        let tendermint_key: H256 = H256::from(rnd_num);
        let account: Address = "4180fc65d613ba7e1a385181a219f1dbfe7bf11d".parse().unwrap();

        Register::new(
            address.parse().unwrap(),
            tendermint_key,
            start_port,
            last_port,
            contract_address,
            account,
            String::from(ETH_URL),
            Credentials::No,
            false,
            1_000_000,
            false,
        )
        .unwrap()
    }

    #[test]
    fn status_correct() -> Result<(), Box<Error>> {
        let reg1_ip = "156.56.34.67";
        let reg1_start_port = 2004;
        let reg1_end_port = 2016;
        let register1 = generate_register(reg1_ip, reg1_start_port, reg1_end_port);

        let reg2_ip = "92.156.134.98";
        let reg2_start_port = 5207;
        let reg2_end_port = 6118;
        let register2 = generate_register(reg2_ip, reg2_start_port, reg2_end_port);

        register1.register(false)?;
        register2.register(false)?;

        let bytes1 = vec![1, 2, 3];
        let cluster_size1 = &2;
        let publisher1 = generate_publisher(bytes1, cluster_size1);

        let bytes2 = vec![1, 2, 3, 4, 5];
        let cluster_size2 = &180;
        let publisher2 = generate_publisher(bytes2, cluster_size2);

        publisher1.publish(false)?;
        publisher2.publish(false)?;

        let _status = get_status(CONTRACT_ADDR.parse().unwrap(), ETH_URL)?;

        //let clusters = status.clusters();
        //let apps = status.enqueued_apps();
        //let nodes = status.ready_nodes();

        //        let cluster = clusters
        //            .iter()
        //            .find(|&cl| cl.code().cluster_size() == cluster_size1);
        //
        //        println!("{:?}", clusters);
        //
        //        assert_eq!(cluster.is_some(), true);
        //
        //        let node1 = nodes.iter().find(|&n| n.ip_addr() == reg1_ip);
        //
        //        assert_eq!(node1.is_some(), true);
        //        let node1 = node1.unwrap();
        //        assert_eq!(node1.start_port(), &reg1_start_port);
        //        assert_eq!(node1.end_port(), &reg1_end_port);
        //
        //        let node2 = nodes.iter().find(|&n| n.ip_addr() == reg2_ip);
        //
        //        assert_eq!(node2.is_some(), true);
        //        let node2 = node2.unwrap();
        //        assert_eq!(node2.start_port(), &reg2_start_port);
        //        assert_eq!(node2.end_port(), &reg2_end_port);

        //        let app = apps.iter().find(|&c| c.cluster_size() == cluster_size2);
        //        assert_eq!(app.is_some(), true);

        Ok(())
    }
}
