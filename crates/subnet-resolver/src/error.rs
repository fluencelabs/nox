use chain_data::ChainDataError;
use libp2p_identity::ParseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResolveSubnetError {
    #[error("error encoding function: '{0}'")]
    EncodeFunction(#[from] ethabi::Error),
    #[error("error sending jsonrpc request: '{0}'")]
    RpcError(#[from] jsonrpsee::core::client::Error),
    #[error(transparent)]
    ChainData(#[from] ChainDataError),
    #[error("getPATs response is empty")]
    Empty,
    #[error("'{1}' from getPATs is not a valid PeerId")]
    InvalidPeerId(#[source] ParseError, &'static str),
    #[error("Invalid deal id '{0}': invalid length")]
    InvalidDealId(String),
}
