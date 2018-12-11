use types::H192;
use web3::types::H256;

#[derive(Debug)]
pub struct Node {
    id: H256,
    address: H192,
    start_port: u16,
    end_port: u16,
    current_port: u16,
}

impl Node {
    pub fn new(id: H256, address: H192, start_port: u16, end_port: u16, current_port: u16) -> Node {
        Node {
            id,
            address,
            start_port,
            end_port,
            current_port,
        }
    }
}
