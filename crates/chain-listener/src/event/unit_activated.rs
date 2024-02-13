use chain_data::EventField::{Indexed, NotIndexed};
use chain_data::{next_opt, ChainData, ChainDataError, ChainEvent, EventField, U256};
use chain_types::{CommitmentId, UnitId};
use ethabi::param_type::ParamType;
use ethabi::Token;
use serde::{Deserialize, Serialize};

/// @dev Emitted when a unit activated. Unit is activated when it returned from deal
/// @param commitmentId Commitment id
/// @param unitId Compute unit id which activated
/// event UnitActivated(
///     bytes32 indexed commitmentId,
///     bytes32 indexed unitId,
///     uint256 startEpoch
/// );

#[derive(Debug, Serialize, Deserialize)]
pub struct UnitActivatedData {
    pub commitment_id: CommitmentId,
    pub unit_id: UnitId,
    pub start_epoch: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnitActivated {
    pub block_number: String,
    pub info: UnitActivatedData,
}

impl UnitActivated {
    pub const EVENT_NAME: &'static str = "UnitActivated";
}

impl ChainData for UnitActivatedData {
    fn event_name() -> &'static str {
        UnitActivated::EVENT_NAME
    }

    fn signature() -> Vec<EventField> {
        vec![
            Indexed(ParamType::FixedBytes(32)), // commitmentId
            Indexed(ParamType::FixedBytes(32)), // unitId
            NotIndexed(ParamType::Uint(256)),   // startEpoch
        ]
    }

    /// Parse data from chain. Accepts data with and without "0x" prefix.
    fn parse(data_tokens: &mut impl Iterator<Item = Token>) -> Result<Self, ChainDataError> {
        let commitment_id = CommitmentId(next_opt(
            data_tokens,
            "commitment_id",
            Token::into_fixed_bytes,
        )?);

        let unit_id = UnitId(next_opt(data_tokens, "unit_id", Token::into_fixed_bytes)?);

        let start_epoch = next_opt(data_tokens, "start_epoch", U256::from_token)?;

        Ok(UnitActivatedData {
            commitment_id,
            unit_id,
            start_epoch,
        })
    }
}

impl ChainEvent<UnitActivatedData> for UnitActivated {
    fn new(block_number: String, info: UnitActivatedData) -> Self {
        Self { block_number, info }
    }
}

#[cfg(test)]
mod test {

    use super::UnitActivated;
    use crate::event::UnitActivatedData;
    use chain_data::{parse_log, ChainData, Log};
    use hex;

    #[tokio::test]
    async fn test_unit_activated_topic() {
        assert_eq!(
            UnitActivatedData::topic(),
            "0x8e4b27eeb3194deef0b3140997e6b82f53eb7350daceb9355268009b92f70add"
        );
    }

    #[tokio::test]
    async fn test_chain_parsing_ok() {
        let data = "0x000000000000000000000000000000000000000000000000000000000000007b00000000000000000000000000000000000000000000000000000000000001c800000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000001c04d94f1e85788b245471c87490f42149b09503fe3af46733e4b5adf94583105".to_string();
        let log = Log {
            data,
            block_number: "0x0".to_string(),
            removed: false,
            topics: vec![
                UnitActivatedData::topic(),
                "0x431688393bc518ef01e11420af290b92f3668dca24fc171eeb11dd15bcefad72".to_string(),
                "0xd33bc101f018e42351fbe2adc8682770d164e27e2e4c6454e0faaf5b8b63b90e".to_string(),
            ],
        };
        let result = parse_log::<UnitActivatedData, UnitActivated>(log);

        assert!(result.is_ok(), "can't parse data: {:?}", result);
        let result = result.unwrap().info;
        assert_eq!(
            hex::encode(result.commitment_id.0),
            "431688393bc518ef01e11420af290b92f3668dca24fc171eeb11dd15bcefad72" // it's the second topic
        );
        assert_eq!(
            hex::encode(result.unit_id.0),
            "d33bc101f018e42351fbe2adc8682770d164e27e2e4c6454e0faaf5b8b63b90e" // it's also the third topic
        );

        assert_eq!(
            result.start_epoch.to_eth(),
            ethabi::ethereum_types::U256::from(123)
        );
    }
}
