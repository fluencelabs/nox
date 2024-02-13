use crate::types::CommitmentId;
use chain_data::{next_opt, parse_chain_data};
use ethabi::ethereum_types::U256;
use ethabi::Token;

/// struct ComputePeer {
///     bytes32 offerId;
///     bytes32 commitmentId;
///     uint256 unitCount;
///     address owner;
/// }
pub struct ComputePeer {
    pub offer_id: Vec<u8>,
    pub commitment_id: Option<CommitmentId>,
    pub unit_count: U256,
    pub owner: String,
}

impl ComputePeer {
    pub fn signature() -> Vec<ethabi::ParamType> {
        vec![
            ethabi::ParamType::FixedBytes(32),
            ethabi::ParamType::FixedBytes(32),
            ethabi::ParamType::Uint(256),
            ethabi::ParamType::Address,
        ]
    }
    pub fn from(data: &str) -> eyre::Result<Self> {
        let mut tokens = parse_chain_data(data, &Self::signature())?.into_iter();
        let offer_id = next_opt(&mut tokens, "offer_id", Token::into_fixed_bytes)?;
        let commitment_id = next_opt(&mut tokens, "commitment_id", Token::into_fixed_bytes)?;

        let commitment_id = if commitment_id.iter().all(|&x| x == 0) {
            None
        } else {
            Some(CommitmentId(commitment_id))
        };

        let unit_count = next_opt(&mut tokens, "unit_count", Token::into_uint)?;
        let owner = next_opt(&mut tokens, "owner", Token::into_address)?;

        Ok(Self {
            offer_id,
            commitment_id,
            unit_count,
            owner: format!("{owner:#x}"),
        })
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn decode_compute_peer_no_commitment() {
        let data = "0xaa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000005b73c5498c1e3b4dba84de0f1833c4a029d90519";
        let compute_peer = super::ComputePeer::from(data);
        assert!(compute_peer.is_ok());
        let compute_peer = compute_peer.unwrap();
        assert_eq!(
            hex::encode(compute_peer.offer_id),
            "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5"
        );
        assert_eq!(compute_peer.commitment_id, None);
        assert_eq!(compute_peer.unit_count, 2.into());
        assert_eq!(
            compute_peer.owner,
            "0x5b73c5498c1e3b4dba84de0f1833c4a029d90519"
        );
    }

    #[tokio::test]
    async fn decode_compute_peer() {
        let data = "0xaa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5aa3046a12a1aac6e840625e6329d70b427328feceedc8d273e5e6454b85633b5000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000005b73c5498c1e3b4dba84de0f1833c4a029d90519";
        let compute_peer = super::ComputePeer::from(data);
        assert!(compute_peer.is_ok());
        let compute_peer = compute_peer.unwrap();
        assert_eq!(
            hex::encode(compute_peer.offer_id),
            "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5"
        );
        assert!(compute_peer.commitment_id.is_some());
        assert_eq!(
            hex::encode(compute_peer.commitment_id.unwrap().0),
            "aa3046a12a1aac6e840625e6329d70b427328feceedc8d273e5e6454b85633b5"
        );
        assert_eq!(compute_peer.unit_count, 10.into());
        assert_eq!(
            compute_peer.owner,
            "0x5b73c5498c1e3b4dba84de0f1833c4a029d90519"
        );
    }
}
