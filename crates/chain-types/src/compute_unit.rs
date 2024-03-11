use ccp_shared::types::CUID;
use chain_data::ChainDataError::InvalidParsedToken;
use chain_data::{next_opt, parse_chain_data, ChainDataError};
use ethabi::ethereum_types::U256;
use ethabi::Token;
use types::DealId;

/// struct ComputeUnitView {
///     bytes32 id;
///     address deal;
///     uint256 startEpoch;
/// }
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ComputeUnit {
    pub id: CUID,
    /// if deal is zero-address, it means the unit is not assigned to any deal
    pub deal: Option<DealId>,
    pub start_epoch: U256,
}

impl ComputeUnit {
    pub fn new(id: CUID, start_epoch: U256) -> Self {
        Self {
            id,
            deal: None,
            start_epoch,
        }
    }
    pub fn signature() -> Vec<ethabi::ParamType> {
        vec![
            ethabi::ParamType::FixedBytes(32),
            ethabi::ParamType::Address,
            ethabi::ParamType::Uint(256),
        ]
    }

    pub fn parse(data: &str) -> Result<Self, ChainDataError> {
        let mut tokens = parse_chain_data(data, &Self::signature())?.into_iter();
        Self::from_tokens(&mut tokens)
    }

    pub fn from_token(token: Token) -> Result<Self, ChainDataError> {
        let mut tokens = next_opt(
            &mut std::iter::once(token),
            "compute_unit",
            Token::into_tuple,
        )?
        .into_iter();
        Self::from_tokens(&mut tokens)
    }

    pub fn from_tokens(
        data_tokens: &mut impl Iterator<Item = Token>,
    ) -> Result<Self, ChainDataError> {
        let id = next_opt(data_tokens, "id", Token::into_fixed_bytes)?;
        let deal = next_opt(data_tokens, "deal", Token::into_address)?;

        // if deal is zero-address, it means the unit is not assigned to any deal
        let deal = if deal.is_zero() {
            None
        } else {
            Some(format!("{deal:#x}").into())
        };

        let start_epoch = next_opt(data_tokens, "start_epoch", Token::into_uint)?;
        Ok(ComputeUnit {
            id: CUID::new(
                id.as_slice()
                    .try_into()
                    .map_err(|_| InvalidParsedToken("id"))?,
            ),
            deal,
            start_epoch,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct PendingUnit {
    pub id: CUID,
    pub start_epoch: U256,
}

impl PendingUnit {
    pub fn new(id: CUID, start_epoch: U256) -> Self {
        Self { id, start_epoch }
    }
}
impl From<ComputeUnit> for PendingUnit {
    fn from(unit: ComputeUnit) -> Self {
        Self {
            id: unit.id,
            start_epoch: unit.start_epoch,
        }
    }
}

#[cfg(test)]
mod tests {
    use ccp_shared::types::CUID;
    use hex::FromHex;

    #[tokio::test]
    async fn decode_compute_unit() {
        let data = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d50000000000000000000000005e3d0fde6f793b3115a9e7f5ebc195bbeed35d6c00000000000000000000000000000000000000000000000000000000000003e8";
        let compute_unit = super::ComputeUnit::parse(data);
        assert!(compute_unit.is_ok());
        let compute_unit = compute_unit.unwrap();

        assert_eq!(
            compute_unit.id,
            <CUID>::from_hex("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5")
                .unwrap()
        );
        assert!(compute_unit.deal.is_some());
        assert_eq!(
            compute_unit.deal.unwrap(),
            "0x5e3d0fde6f793b3115a9e7f5ebc195bbeed35d6c"
        );
        assert_eq!(compute_unit.start_epoch, 1000.into());
    }

    #[tokio::test]
    async fn decode_compute_unit_no_deal() {
        let data = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003e8";
        let compute_unit = super::ComputeUnit::parse(data);
        assert!(compute_unit.is_ok());
        let compute_unit = compute_unit.unwrap();
        assert_eq!(
            compute_unit.id,
            <CUID>::from_hex("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5")
                .unwrap()
        );
        assert!(compute_unit.deal.is_none());
        assert_eq!(compute_unit.start_epoch, 1000.into());
    }
}
