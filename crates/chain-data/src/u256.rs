use ethabi::Token;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct U256 {
    bytes: Vec<u8>,
}

impl U256 {
    pub fn from_bytes(bs: &[u8; 32]) -> Self {
        U256 { bytes: bs.to_vec() }
    }

    pub fn to_eth(&self) -> ethabi::ethereum_types::U256 {
        ethabi::ethereum_types::U256::from_little_endian(&self.bytes)
    }

    pub fn to_u64_trunc(&self) -> u64 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.bytes[0..8]);
        u64::from_le_bytes(bytes)
    }

    pub fn from_eth(num: ethabi::ethereum_types::U256) -> U256 {
        let bytes = num
            .0
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<_>>();
        U256 { bytes }
    }

    pub fn from_token(token: Token) -> Option<Self> {
        token.into_uint().map(Self::from_eth)
    }
}
