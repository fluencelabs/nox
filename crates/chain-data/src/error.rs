use thiserror::Error;
#[derive(Debug, Error)]
pub enum ChainDataError {
    #[error("empty data, nothing to parse")]
    Empty,
    #[error("missing token for field '{0}'")]
    MissingParsedToken(&'static str),
    #[error("invalid token for field '{0}'")]
    InvalidParsedToken(&'static str),
    #[error("invalid compute peer id: '{0}'")]
    InvalidComputePeerId(#[from] libp2p_identity::ParseError),
    #[error("data is not a valid hex: '{0}'")]
    DecodeHex(#[source] hex::FromHexError),
    #[error(transparent)]
    EthError(#[from] ethabi::Error),
    #[error("Invalid token size")]
    InvalidTokenSize,
}
