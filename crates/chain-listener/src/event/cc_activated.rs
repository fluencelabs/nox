use chain_data::EventField::{Indexed, NotIndexed};
use chain_data::{
    next_opt, parse_peer_id, ChainData, ChainDataError, ChainEvent, EventField, U256,
};
use ethabi::param_type::ParamType;
use ethabi::Token;
use fluence_libp2p::peerid_serializer;
use libp2p_identity::PeerId;
use serde::{Deserialize, Serialize};

/// Corresponding Solidity type:
/// ```solidity
///event CapacityCommitmentActivated(
///    bytes32 indexed peerId,
///    bytes32 indexed commitmentId,
///    uint256 endEpoch,
///    bytes32[] unitIds,
///);
/// ```

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitmentId(pub Vec<u8>);
#[derive(Debug, Serialize, Deserialize)]
pub struct UnitId(pub Vec<u8>);

#[derive(Debug, Serialize, Deserialize)]
pub struct CCActivatedData {
    #[serde(with = "peerid_serializer")]
    pub peer_id: PeerId,
    pub commitment_id: CommitmentId,
    pub end_epoch: U256,
    pub unit_ids: Vec<UnitId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CCActivated {
    pub block_number: String,
    pub info: CCActivatedData,
}

impl CCActivated {
    pub const EVENT_NAME: &'static str = "CapacityCommitmentActivated";
}

impl ChainData for CCActivatedData {
    fn event_name() -> &'static str {
        CCActivated::EVENT_NAME
    }

    fn signature() -> Vec<EventField> {
        vec![
            Indexed(ParamType::FixedBytes(32)), // peerId
            Indexed(ParamType::FixedBytes(32)), // commitmentId
            NotIndexed(ParamType::Uint(256)),   // endEpoch
            NotIndexed(ParamType::Array(Box::new(ParamType::FixedBytes(32)))), // unitIds
        ]
    }

    /// Parse data from chain. Accepts data with and without "0x" prefix.
    fn parse(data_tokens: &mut impl Iterator<Item = Token>) -> Result<Self, ChainDataError> {
        let peer_id = next_opt(data_tokens, "peer_id", Token::into_fixed_bytes)?;
        let peer_id = parse_peer_id(peer_id)?;

        let commitment_id = CommitmentId(next_opt(
            data_tokens,
            "commitment_id",
            Token::into_fixed_bytes,
        )?);

        let end_epoch = next_opt(data_tokens, "end_epoch", U256::from_token)?;

        let unit_ids: Vec<Vec<u8>> = next_opt(data_tokens, "unit_ids", |t| {
            t.into_array()?
                .into_iter()
                .map(Token::into_fixed_bytes)
                .collect()
        })?;
        let unit_ids = unit_ids.into_iter().map(UnitId).collect();

        Ok(CCActivatedData {
            peer_id,
            commitment_id,
            end_epoch,
            unit_ids,
        })
    }
}

impl ChainEvent<CCActivatedData> for CCActivated {
    fn new(block_number: String, info: CCActivatedData) -> Self {
        Self { block_number, info }
    }
}

#[cfg(test)]
mod test {

    use super::{CCActivated, CCActivatedData};
    use chain_data::{parse_log, ChainData, Log};
    use hex;

    #[tokio::test]
    async fn test_cc_activated_topic() {
        assert_eq!(
            CCActivatedData::topic(),
            "0xcd92fc03744bba25ad966bdc1127f8996e70c551d1ee4a88ce7fb0e596069649"
        );
    }

    #[tokio::test]
    async fn test_chain_parsing_ok() {
        let data = "0x00000000000000000000000000000000000000000000000000000000009896800000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000a4c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc94c5951c05568dac529f66b98217d93627dd10f3070a928b427ad82cdadc72cc9".to_string();
        let log = Log {
            data,
            block_number: "0x0".to_string(),
            removed: false,
            topics: vec![
                CCActivatedData::topic(),
                "0x246cd65bc58db104674f76c9b1340eb16881d9ef90e33d4b1086ebd334f4002d".to_string(),
                "0xd6996a1d0950671fa4ae2642e9bfdb7e4c7832a35c640cdb47ecb8b8002b77b5".to_string(),
            ],
        };
        let result = parse_log::<CCActivatedData, CCActivated>(log);

        assert!(result.is_ok(), "can't parse data: {:?}", result);
        let result = result.unwrap().info;
        assert_eq!(
            result.peer_id.to_string(),
            "12D3KooWCGZ6t8by5ag5YMQW4k3HoPLaKdN5rB9DhAmDUeG8dj1N"
        );

        assert_eq!(
            hex::encode(result.commitment_id.0),
            "d6996a1d0950671fa4ae2642e9bfdb7e4c7832a35c640cdb47ecb8b8002b77b5"
        );
        assert_eq!(
            result.end_epoch.to_eth(),
            ethabi::ethereum_types::U256::exp10(7)
        );

        assert_eq!(result.unit_ids.len(), 10);
    }
}
