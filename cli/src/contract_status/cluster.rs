use contract_status::code::Code;
use types::H192;
use web3::types::{H256, U256};

#[derive(Debug)]
pub struct ClusterMember {
    id: H256,
    address: H192,
    port: u16,
}

impl ClusterMember {
    pub fn new(id: H256, address: H192, port: u16) -> ClusterMember {
        ClusterMember { id, address, port }
    }
}

#[derive(Debug)]
pub struct Cluster {
    id: H256,
    genesis_time: U256,
    code: Code,
    cluster_members: Vec<ClusterMember>,
}

impl Cluster {
    pub fn new(
        id: H256,
        genesis_time: U256,
        code: Code,
        cluster_members: Vec<ClusterMember>,
    ) -> Cluster {
        Cluster {
            id,
            genesis_time,
            code,
            cluster_members,
        }
    }
}
