extern crate web3;
extern crate clap;

pub mod app {

    use clap::{Arg, App, ArgMatches};
    use web3::types::Address;
    use console::style;

    pub struct Publisher<'a> {
        pub path: &'a str,
        pub contract_address: Address,
        pub account: Address,
        pub swarm_url: &'a str,
        pub eth_url: &'a str,
        pub password: Option<&'a str>,
        pub cluster_size: u64
    }

    impl<'a> Publisher<'a> {
        fn new(path: &'a str, contract_address: Address, account: Address, swarm_url: &'a str,
               eth_url: &'a str, password: Option<&'a str>, cluster_size: u64) -> Self {
            Publisher {
                path, contract_address, account, swarm_url, eth_url, password, cluster_size
            }
        }
    }

    pub fn init() -> Result<Publisher<'static>, Box<std::error::Error>> {
        let matches = App::new(format!("{}", style("Fluence Code Publisher").blue().bold()))
            .version("0.1.0")
            .author(format!("{}", style("Fluence Labs").blue().bold()).as_str())
            .about(format!("{}", style("Console utility for deploying code to fluence cluster").blue().bold()).as_str())
            .arg(Arg::with_name("path")
                .required(true)
                .takes_value(true)
                .index(1)
                .help("path to compiled `wasm` code"))
            .arg(Arg::with_name("account").alias("account")
                .required(true).alias("account").long("account").short("a")
                .takes_value(true)
                .help("ethereum account without `0x`"))
            .arg(Arg::with_name("contract_address").alias("contract_address")
                .required(true)
                .takes_value(true)
                .index(2)
                .help("deployer contract address without `0x`"))
            .arg(Arg::with_name("swarm_url").alias("swarm_url").long("swarm_url").short("s")
                .required(false)
                .takes_value(true)
                .help("http address to swarm node")
                .default_value("http://localhost:8500/")) //todo: use public gateway
            .arg(Arg::with_name("eth_url").alias("eth_url").long("eth_url").short("e")
                .required(false)
                .takes_value(true)
                .help("http address to ethereum node")
                .default_value("http://localhost:8545/")) //todo: use public node or add light client
            .arg(Arg::with_name("password").alias("password").long("password").short("p")
                .required(false)
                .takes_value(true)
                .help("password to unlock account in ethereum client"))
            .arg(Arg::with_name("cluster_size").alias("cluster_size").long("cluster_size").short("cs")
                .required(false)
                .takes_value(true)
                .default_value("3")
                .help("cluster's size that needed to deploy this code"))
            .get_matches();

        let path = matches.value_of("path");
        let path = path.unwrap();

        let contract_address = matches.value_of("contract_address").unwrap();
        let contract_address: Address = contract_address.parse()?;

        let account = matches.value_of("account").unwrap();
        let account: Address = account.parse()?;

        let swarm_url = matches.value_of("swarm_url").unwrap();
        let eth_url = matches.value_of("eth_url").unwrap();

        let password = matches.value_of("password");

        let cluster_size: u64 = matches.value_of("cluster_size").unwrap().parse()?;
        if cluster_size < 1 || cluster_size > 255 {
            panic!("Invalid number: {}. Must be from 1 to 255.");
        }

        Ok(Publisher::new(path, contract_address, account, swarm_url, eth_url, password, cluster_size))
    }
}