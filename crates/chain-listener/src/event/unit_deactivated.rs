use chain_data::ChainDataError::InvalidTokenSize;
use chain_data::EventField::Indexed;
use chain_data::{next_opt, ChainData, ChainDataError, ChainEvent, EventField};
use chain_types::CommitmentId;
use core_manager::CUID;
use ethabi::param_type::ParamType;
use ethabi::Token;
use serde::{Deserialize, Serialize};

/// @dev Emitted when a unit deactivated. Unit is deactivated when it moved to deal
/// @param commitmentId Commitment id
/// @param unitId Compute unit id which deactivated
/// event UnitDeactivated(
///     bytes32 indexed commitmentId,
///     bytes32 indexed unitId
/// );

#[derive(Debug, Serialize, Deserialize)]
pub struct UnitDeactivatedData {
    pub commitment_id: CommitmentId,
    pub unit_id: CUID,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnitDeactivated {
    pub block_number: String,
    pub info: UnitDeactivatedData,
}

impl UnitDeactivated {
    pub const EVENT_NAME: &'static str = "UnitDeactivated";
}

impl ChainData for UnitDeactivatedData {
    fn event_name() -> &'static str {
        UnitDeactivated::EVENT_NAME
    }

    fn signature() -> Vec<EventField> {
        vec![
            Indexed(ParamType::FixedBytes(32)), // commitmentId
            Indexed(ParamType::FixedBytes(32)), // unitId
        ]
    }

    /// Parse data from chain. Accepts data with and without "0x" prefix.
    fn parse(data_tokens: &mut impl Iterator<Item = Token>) -> Result<Self, ChainDataError> {
        let commitment_id = CommitmentId(next_opt(
            data_tokens,
            "commitment_id",
            Token::into_fixed_bytes,
        )?);

        let unit_id = next_opt(data_tokens, "unit_id", Token::into_fixed_bytes)?;

        Ok(UnitDeactivatedData {
            commitment_id,
            unit_id: CUID::new(unit_id.try_into().map_err(|_| InvalidTokenSize)?),
        })
    }
}

impl ChainEvent<UnitDeactivatedData> for UnitDeactivated {
    fn new(block_number: String, info: UnitDeactivatedData) -> Self {
        Self { block_number, info }
    }
}

#[cfg(test)]
mod test {

    use crate::event::unit_deactivated::UnitDeactivated;
    use crate::event::UnitDeactivatedData;
    use chain_data::{parse_log, ChainData, Log};
    use core_manager::CUID;

    use hex::FromHex;

    #[tokio::test]
    async fn test_unit_activated_topic() {
        assert_eq!(
            UnitDeactivatedData::topic(),
            "0xbd9cde1bbc961036d34368ae328c38917036a98eacfb025a1ff6d2c6235d0a14"
        );
    }

    #[tokio::test]
    async fn test_chain_parsing_ok() {
        let data = "0x".to_string();
        let log = Log {
            data,
            block_number: "0x0".to_string(),
            removed: false,
            topics: vec![
                UnitDeactivatedData::topic(),
                "0x91cfcc4a139573b08646960be31b278152ef3480710ab15d9b39262be37038a1".to_string(),
                "0xf3660ca1eaf461cbbb5e1d06ade6ba4a9a503c0d680ba825e09cddd3f9b45fc6".to_string(),
            ],
        };
        let result = parse_log::<UnitDeactivatedData, UnitDeactivated>(log);

        assert!(result.is_ok(), "can't parse data: {:?}", result);
        let result = result.unwrap().info;
        assert_eq!(
            hex::encode(result.commitment_id.0),
            "91cfcc4a139573b08646960be31b278152ef3480710ab15d9b39262be37038a1" // it's the second topic
        );
        assert_eq!(
            result.unit_id,
            <CUID>::from_hex("f3660ca1eaf461cbbb5e1d06ade6ba4a9a503c0d680ba825e09cddd3f9b45fc6")
                .unwrap() // it's also the third topic
        );
    }
}
