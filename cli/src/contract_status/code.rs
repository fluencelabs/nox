use web3::types::H256;

#[derive(Debug)]
pub struct Code {
    storage_hash: H256,
    storage_receipt: H256,
    cluster_size: u8,
}

impl Code {
    pub fn new(storage_hash: H256, storage_receipt: H256, cluster_size: u8) -> Code {
        Code {
            storage_hash,
            storage_receipt,
            cluster_size,
        }
    }
}
