use chain_data::ChainDataError::InvalidTokenSize;
use chain_data::EventField::{Indexed, NotIndexed};
use chain_data::{next_opt, parse_peer_id, ChainData, ChainDataError, ChainEvent, EventField};
use chain_types::CommitmentId;
use core_manager::CUID;
use ethabi::ethereum_types::U256;
use ethabi::param_type::ParamType;
use ethabi::Token;
use libp2p_identity::PeerId;
use serde::{Deserialize, Serialize};
use types::peer_id;

/// @dev Emitted when a commitment is activated. Commitment can be activated only if delegator deposited collateral.
/// @param peerId Peer id which linked to the commitment
/// @param commitmentId Commitment id which activated
/// @param startEpoch The start epoch of the commitment
/// @param endEpoch The end epoch of the commitment
/// @param unitIds Compute unit ids which linked to the commitment
/// event CommitmentActivated(
///     bytes32 indexed peerId,
///     bytes32 indexed commitmentId,
///     uint256 startEpoch,
///     uint256 endEpoch,
///     bytes32[] unitIds
/// );
#[derive(Debug, Serialize, Deserialize)]
pub struct CommitmentActivatedData {
    #[serde(
        serialize_with = "peer_id::serde::serialize",
        deserialize_with = "peer_id::serde::deserialize"
    )]
    pub peer_id: PeerId,
    pub commitment_id: CommitmentId,
    pub start_epoch: U256,
    pub end_epoch: U256,
    pub unit_ids: Vec<CUID>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitmentActivated {
    pub block_number: String,
    pub info: CommitmentActivatedData,
}

impl CommitmentActivated {
    pub const EVENT_NAME: &'static str = "CommitmentActivated";
}

impl ChainData for CommitmentActivatedData {
    fn event_name() -> &'static str {
        CommitmentActivated::EVENT_NAME
    }

    fn signature() -> Vec<EventField> {
        vec![
            Indexed(ParamType::FixedBytes(32)), // peerId
            Indexed(ParamType::FixedBytes(32)), // commitmentId
            NotIndexed(ParamType::Uint(256)),   // startEpoch
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

        let start_epoch = next_opt(data_tokens, "start_epoch", Token::into_uint)?;
        let end_epoch = next_opt(data_tokens, "end_epoch", Token::into_uint)?;

        let units: Vec<Vec<u8>> = next_opt(data_tokens, "unit_ids", |t| {
            t.into_array()?
                .into_iter()
                .map(Token::into_fixed_bytes)
                .collect()
        })?;

        let mut unit_ids = vec![];
        for cu in units {
            unit_ids.push(CUID::new(cu.try_into().map_err(|_| InvalidTokenSize)?));
        }

        Ok(CommitmentActivatedData {
            peer_id,
            commitment_id,
            start_epoch,
            end_epoch,
            unit_ids,
        })
    }
}

impl ChainEvent<CommitmentActivatedData> for CommitmentActivated {
    fn new(block_number: String, info: CommitmentActivatedData) -> Self {
        Self { block_number, info }
    }
}

#[cfg(test)]
mod test {

    use super::{CommitmentActivated, CommitmentActivatedData};
    use chain_data::{parse_log, ChainData, Log};
    use core_manager::CUID;
    use hex;
    use hex::FromHex;

    #[tokio::test]
    async fn test_cc_activated_topic() {
        assert_eq!(
            CommitmentActivatedData::topic(),
            "0x0b0a4688a90d1b24732d05ddf4925af69f02cd7d9a921b1cdcd4a7c2b6d57d68"
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
                CommitmentActivatedData::topic(),
                "0xc586dcbfc973643dc5f885bf1a38e054d2675b03fe283a5b7337d70dda9f7171".to_string(),
                "0x27e42c090aa007a4f2545547425aaa8ea3566e1f18560803ac48f8e98cb3b0c9".to_string(),
            ],
        };
        let result = parse_log::<CommitmentActivatedData, CommitmentActivated>(log);

        assert!(result.is_ok(), "can't parse data: {:?}", result);
        let result = result.unwrap().info;
        assert_eq!(
            result.peer_id.to_string(),
            "12D3KooWP7RkvkBhbe7ATd451zxTifzF6Gm1uzCDadqQueET7EMe" // it's also the second topic
        );

        assert_eq!(
            hex::encode(result.commitment_id.0),
            "27e42c090aa007a4f2545547425aaa8ea3566e1f18560803ac48f8e98cb3b0c9" // it's the third topic
        );
        assert_eq!(result.start_epoch, 123.into());
        assert_eq!(result.end_epoch, 456.into());

        assert_eq!(result.unit_ids.len(), 1);
        assert_eq!(
            result.unit_ids[0],
            <CUID>::from_hex("c04d94f1e85788b245471c87490f42149b09503fe3af46733e4b5adf94583105")
                .unwrap()
        )
    }
}
