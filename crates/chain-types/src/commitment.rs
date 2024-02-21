use crate::commitment_status::CommitmentStatus;
use chain_data::LogParseError::MissingParsedToken;
use chain_data::{next_opt, parse_chain_data, ChainDataError};
use ethabi::ethereum_types::U256;
use ethabi::{ParamType, Token};
use eyre::eyre;

///    struct CommitmentView {
///         CCStatus status;
///         bytes32 peerId;
///         uint256 collateralPerUnit;
///         uint256 unitCount;
///         uint256 startEpoch;
///         uint256 endEpoch;
///         uint256 rewardDelegatorRate;
///         address delegator;
///         uint256 totalCUFailCount;
///         uint256 failedEpoch;
///         uint256 exitedUnitCount;
///     }
pub struct Commitment {
    pub status: CommitmentStatus,
    pub start_epoch: U256,
    pub end_epoch: U256,
}

impl Commitment {
    pub fn signature() -> Vec<ParamType> {
        vec![
            ParamType::Uint(8),        // CCStatus status
            ParamType::FixedBytes(32), // bytes32 peerId
            ParamType::Uint(256),      // uint256 collateralPerUnit
            ParamType::Uint(256),      // uint256 unitCount
            ParamType::Uint(256),      // uint256 startEpoch
            ParamType::Uint(256),      // uint256 endEpoch
            ParamType::Uint(256),      // uint256 rewardDelegatorRate
            ParamType::Address,        // address delegator
            ParamType::Uint(256),      // uint256 totalCUFailCount
            ParamType::Uint(256),      // uint256 failedEpoch
            ParamType::Uint(256),      // uint256 exitedUnitCount
        ]
    }

    pub fn from(data: &str) -> Result<Self, ChainDataError> {
        let mut tokens = parse_chain_data(data, &Self::signature())?.into_iter();
        let status = next_opt(
            &mut tokens,
            "commitment_status",
            CommitmentStatus::from_token,
        )?;
        let mut tokens = tokens.skip(3);
        let start_epoch = next_opt(&mut tokens, "start_epoch", Token::into_uint)?;
        let end_epoch = next_opt(&mut tokens, "end_epoch", Token::into_uint)?;
        Ok(Commitment {
            status,
            start_epoch,
            end_epoch,
        })
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn decode_commitment() {
        let data = "0x00000000000000000000000000000000000000000000000000000000000000016497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000012c00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let commitment = super::Commitment::from(data);
        assert!(commitment.is_ok());
        let commitment = commitment.unwrap();
        assert_eq!(commitment.status, super::CommitmentStatus::WaitDelegation);
        assert_eq!(commitment.start_epoch, 0.into());
        assert_eq!(commitment.end_epoch, 300.into());
    }
}
