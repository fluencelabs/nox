use ccp_shared::types::CUID;
use chain_data::ChainDataError::InvalidTokenSize;
use chain_data::EventField::{Indexed, NotIndexed};
use chain_data::{next_opt, parse_peer_id, ChainData, ChainDataError, ChainEvent, EventField};
use ethabi::ethereum_types::U256;
use ethabi::param_type::ParamType;
use ethabi::Token;
use types::DealId;

/// Corresponding Solidity type:
/// ```solidity
/// struct CIDV1 {
///     bytes4 prefixes;
///     bytes32 hash;
/// }
///
/// event ComputeUnitMatched(
///     bytes32 indexed peerId,
///     address deal
///     bytes32 unitId,
///     uint256 dealCreationBlock,
///     CIDV1 appCID
/// );
/// ```

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DealMatchedData {
    compute_peer: String,
    pub deal_id: DealId,
    unit_id: CUID,
    deal_creation_block: U256,
    app_cid: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DealMatched {
    block_number: String,
    pub info: DealMatchedData,
}

impl DealMatched {
    pub const EVENT_NAME: &'static str = "ComputeUnitMatched";
}

impl ChainData for DealMatchedData {
    fn event_name() -> &'static str {
        DealMatched::EVENT_NAME
    }

    fn signature() -> Vec<EventField> {
        vec![
            // compute_provider
            Indexed(ParamType::FixedBytes(32)),
            // deal
            NotIndexed(ParamType::Address),
            // unit_id
            NotIndexed(ParamType::FixedBytes(32)),
            // deal_creation_block
            NotIndexed(ParamType::Uint(256)),
            // app_cid
            NotIndexed(ParamType::Tuple(vec![
                // prefixes
                ParamType::FixedBytes(4),
                // hash
                ParamType::FixedBytes(32),
            ])),
        ]
    }

    /// Parse data from chain. Accepts data with and without "0x" prefix.
    fn parse(data_tokens: &mut impl Iterator<Item = Token>) -> Result<Self, ChainDataError> {
        let tokens = &mut data_tokens.into_iter();

        let compute_peer = next_opt(tokens, "compute_peer", Token::into_fixed_bytes)?;
        let compute_peer = parse_peer_id(compute_peer)?.to_string();

        let deal = next_opt(tokens, "deal", Token::into_address)?;
        let unit_id = next_opt(tokens, "unit_id", Token::into_fixed_bytes)?;
        let deal_creation_block = next_opt(tokens, "deal_creation_block", Token::into_uint)?;

        let app_cid = &mut next_opt(tokens, "app_cid", Token::into_tuple)?.into_iter();
        let cid_prefixes = next_opt(app_cid, "app_cid.prefixes", Token::into_fixed_bytes)?;
        let cid_hash = next_opt(app_cid, "app_cid.cid_hash", Token::into_fixed_bytes)?;
        let cid_bytes = [cid_prefixes, cid_hash].concat();
        let app_cid = libipld::Cid::read_bytes(cid_bytes.as_slice())
            .map_err(|_| ChainDataError::InvalidParsedToken("app_cid"))?
            .to_string();

        Ok(DealMatchedData {
            compute_peer,
            deal_id: format!("{deal:#x}").into(),
            unit_id: CUID::new(unit_id.try_into().map_err(|_| InvalidTokenSize)?),
            deal_creation_block,
            app_cid,
        })
    }
}

impl ChainEvent<DealMatchedData> for DealMatched {
    fn new(block_number: String, info: DealMatchedData) -> Self {
        Self { block_number, info }
    }
}

#[cfg(test)]
mod tests {
    use super::{DealMatched, DealMatchedData};
    use chain_data::{parse_log, parse_peer_id, ChainData, Log};
    use hex_utils::decode_hex;

    #[test]
    fn topic() {
        assert_eq!(
            DealMatchedData::topic(),
            String::from("0xb1c5a9179c3104a43de668491f14c45778f00ec34d5deee023af204820483bdb")
        );
    }

    #[test]
    fn peer_id() {
        let bytes = [
            88, 198, 255, 218, 126, 170, 188, 84, 84, 39, 255, 137, 18, 55, 7, 139, 121, 207, 149,
            42, 196, 115, 102, 160, 4, 47, 227, 62, 7, 53, 189, 15,
        ];
        let peer_id = parse_peer_id(bytes.into()).expect("parse peer_id from Token");
        assert_eq!(
            peer_id.to_string(),
            String::from("12D3KooWFnv3Qc25eKpTDCNBoW1jXHMHHHSzcJoPkHai1b2dHNra")
        );

        let hex = "0x7a82a5feefcaad4a89c689412031e5f87c02b29e3fced583be5f05c7077354b7";
        let bytes = decode_hex(hex).expect("parse peer_id from hex");
        let peer_id = parse_peer_id(bytes).expect("parse peer_id from Token");
        assert_eq!(
            peer_id.to_string(),
            String::from("12D3KooWJ4bTHirdTFNZpCS72TAzwtdmavTBkkEXtzo6wHL25CtE")
        );
    }

    #[test]
    fn parse() {
        let data1 = "0x000000000000000000000000ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b9900000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000500155122000000000000000000000000000000000000000000000000000000000ae5c519332925f31f747a4edd958fb5b0791b10383ec6d5e77e2264f211e09e300000000000000000000000000000000000000000000000000000000000000036c9d5e8bcc73a422dd6f968f13cd6fc92ccd5609b455cf2c7978cbc694297853fef3b95696986bf289166835e05f723f0fdea97d2bc5fea0ebbbf87b6a866cfa5a5a0f4fa4d41a4f976e799895cce944d5080041dba7d528d30e81c67973bac3".to_string();
        let data2 = "0x00000000000000000000000067b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000560155122000000000000000000000000000000000000000000000000000000000ae5c519332925f31f747a4edd958fb5b0791b10383ec6d5e77e2264f211e09e300000000000000000000000000000000000000000000000000000000000000036c9d5e8bcc73a422dd6f968f13cd6fc92ccd5609b455cf2c7978cbc694297853fef3b95696986bf289166835e05f723f0fdea97d2bc5fea0ebbbf87b6a866cfa5a5a0f4fa4d41a4f976e799895cce944d5080041dba7d528d30e81c67973bac3".to_string();
        let log1 = Log {
            data: data1,
            block_number: "0x0".to_string(),
            removed: false,
            topics: vec![
                DealMatchedData::topic(),
                "0x7a82a5feefcaad4a89c689412031e5f87c02b29e3fced583be5f05c7077354b7".to_string(),
            ],
        };
        let log2 = Log {
            data: data2,
            block_number: "0x1".to_string(),
            removed: false,
            topics: vec![
                DealMatchedData::topic(),
                "0x7a82a5feefcaad4a89c689412031e5f87c02b29e3fced583be5f05c7077354b7".to_string(),
            ],
        };

        let m =
            parse_log::<DealMatchedData, DealMatched>(log1).expect("error parsing Match from log");
        assert_eq!(m.block_number, "0x0");
        let m = m.info;
        assert_eq!(
            m.compute_peer,
            "12D3KooWJ4bTHirdTFNZpCS72TAzwtdmavTBkkEXtzo6wHL25CtE"
        );
        assert_eq!(m.deal_id, "0xFfA0611a099AB68AD7C3C67B4cA5bbBEE7a58B99");
        assert_eq!(
            m.unit_id.to_string(),
            "00000000000000000000000000000000000000000000000000000000000000a0"
        );
        assert_eq!(m.deal_creation_block, 80.into());
        assert_eq!(
            m.app_cid,
            "bafkreifolrizgmusl4y7or5e5xmvr623a6i3ca4d5rwv457cezhschqj4m"
        );

        let m =
            parse_log::<DealMatchedData, DealMatched>(log2).expect("error parsing Match from log");
        assert_eq!(m.block_number, "0x1");
        let m = m.info;
        assert_eq!(
            m.compute_peer,
            "12D3KooWJ4bTHirdTFNZpCS72TAzwtdmavTBkkEXtzo6wHL25CtE"
        );
        assert_eq!(m.deal_id, "0x67b2AD3866429282e16e55B715d12A77F85B7CE8");
        assert_eq!(
            m.unit_id.to_string(),
            "00000000000000000000000000000000000000000000000000000000000000a0"
        );
        assert_eq!(m.deal_creation_block, 86.into());
        assert_eq!(
            m.app_cid,
            "bafkreifolrizgmusl4y7or5e5xmvr623a6i3ca4d5rwv457cezhschqj4m"
        );
    }
}
