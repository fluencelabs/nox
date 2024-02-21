use chain_data::ChainDataError;
use jsonrpsee::core::client::{Error as RPCError, Error};
use std::string::FromUtf8Error;

use hex_utils::decode_hex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectorError {
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
}

pub fn process_response<T>(response: Result<T, RPCError>) -> Result<T, ConnectorError> {
    match response {
        Ok(data) => Ok(data),
        Err(err) => match err {
            Error::Call(e) => {
                let code = e.code();
                let message = e.message().to_string();
                let data = match e.data() {
                    Some(data) => String::from_utf8(decode_hex(&data.to_string())?)?,
                    None => "".to_string(),
                };
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
