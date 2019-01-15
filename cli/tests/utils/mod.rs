use std::error::Error;

use ethkey::Secret;
use rand::prelude::*;
use web3::types::*;

use fluence::credentials::Credentials;

use fluence::register::Register;

pub fn generate_register(credentials: Credentials) -> Register {
    let contract_address: Address = "9995882876ae612bfd829498ccd73dd962ec950a".parse().unwrap();

    let mut rng = rand::thread_rng();
    let rnd_num: u64 = rng.gen();

    let tendermint_key: H256 = H256::from(rnd_num);
    let account: Address = "4180fc65d613ba7e1a385181a219f1dbfe7bf11d".parse().unwrap();

    Register::new(
        "127.0.0.1".parse().unwrap(),
        tendermint_key,
        25006,
        25100,
        contract_address,
        account,
        String::from("http://localhost:8545/"),
        credentials,
        false,
        1_000_000,
        false,
    )
    .unwrap()
}