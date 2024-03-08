use chain_data::ChainDataError;
use jsonrpsee::core::client::{Error as RPCError, Error};
use jsonrpsee::types::ErrorObjectOwned;
use std::string::FromUtf8Error;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("IPC internal network error: {0}")]
    IpcInternalNetworkError(#[source] ErrorObjectOwned),
    #[error("RPC error: {0}")]
    RpcError(#[from] RPCError),
    #[error("RPC call error: code: {code}, message: {message}, data: {data}")]
    RpcCallError {
        /// Code
        code: i32,
        /// Message
        message: String,
        /// Optional data
        data: String,
    },
    #[error("Failed to parse chain data: {0}")]
    ParseChainDataFailed(#[from] ChainDataError),
    #[error("Failed to parse address: {0}")]
    AddressParseError(#[from] clarity::Error),
    #[error("data is not a valid hex: '{0}'")]
    DecodeHex(#[from] hex::FromHexError),
    #[error("data is not a valid string: '{0}'")]
    DecodeData(#[from] FromUtf8Error),
    #[error("Failed to parse baseFeePerGas: {0}")]
    InvalidBaseFeePerGas(String),
    #[error("Invalid transaction nonce: {0}")]
    InvalidNonce(String),
    #[error("Invalid gas limit: {0}")]
    InvalidGasLimit(String),
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),
}

pub fn process_response<T>(response: Result<T, RPCError>) -> Result<T, ConnectorError> {
    match response {
        Ok(data) => Ok(data),
        Err(err) => match err {
            Error::Call(e) => {
                let code = e.code();
                let message = e.message().to_string();
                let data = match e.data() {
                    Some(data) => serde_json::from_str(data.get())?,
                    None => "".to_string(),
                };

                if message.to_lowercase().contains("tendermint rpc error")
                    || message.to_lowercase().contains("connection reset by peer")
                {
                    return Err(ConnectorError::IpcInternalNetworkError(e));
                }

                Err(ConnectorError::RpcCallError {
                    code,
                    message,
                    data,
                })
            }
            _ => Err(ConnectorError::RpcError(err)),
        },
    }
}
