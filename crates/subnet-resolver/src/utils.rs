use crate::error::ChainDataError;
use ethabi::Token;
use libp2p_identity::{ParseError, PeerId};

/// Static prefix of the PeerId. Protobuf encoding + multihash::identity + length and so on.
pub(crate) const PEER_ID_PREFIX: &[u8] = &[0, 36, 8, 1, 18, 32];

pub(crate) fn parse_peer_id(bytes: Vec<u8>) -> Result<PeerId, ParseError> {
    let peer_id = [PEER_ID_PREFIX, &bytes].concat();

    PeerId::from_bytes(&peer_id)
}

pub(crate) fn decode_hex(h: &str) -> Result<Vec<u8>, hex::FromHexError> {
    let h = h.trim_start_matches("0x");
    hex::decode(h)
}

pub(crate) fn next_opt<T>(
    data_tokens: &mut impl Iterator<Item = Token>,
    name: &'static str,
    f: impl Fn(Token) -> Option<T>,
) -> Result<T, ChainDataError> {
    let next = data_tokens
        .next()
        .ok_or(ChainDataError::MissingParsedToken(name))?;
    let parsed = f(next).ok_or(ChainDataError::InvalidParsedToken(name))?;

    Ok(parsed)
}
