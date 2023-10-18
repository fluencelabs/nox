use libp2p_identity::ParseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChainDataError {
    #[error("empty data, nothing to parse")]
    Empty,
    #[error("missing token for field '{0}'")]
    MissingParsedToken(&'static str),
    #[error("invalid token for field '{0}'")]
    InvalidParsedToken(&'static str),
    #[error("data is not a valid hex: '{0}'")]
    DecodeHex(#[source] hex::FromHexError),
    #[error(transparent)]
    EthError(#[from] ethabi::Error),
}

#[derive(Error, Debug)]
pub enum ResolveSubnetError {
    #[error("error encoding function: '{0}'")]
    EncodeFunction(#[from] ethabi::Error),
    #[error("error sending jsonrpc request: '{0}'")]
    RpcError(#[from] jsonrpsee::core::error::Error),
    #[error(transparent)]
    ChainData(#[from] ChainDataError),
    #[error("getPATs response is empty")]
    Empty,
    #[error("'{1}' from getPATs is not a valid PeerId")]
    InvalidPeerId(#[source] ParseError, &'static str),
    #[error("Invalid deal id '{0}': invalid length")]
    InvalidDealId(String),
}
