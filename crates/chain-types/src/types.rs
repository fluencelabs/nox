use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct CommitmentId(pub Vec<u8>);

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct UnitId(pub Vec<u8>);

pub struct Proof {
    pub unit_id: UnitId,
    pub local_unit_nonce: Vec<u8>,
    pub target_hash: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalNonce(pub Vec<u8>);
