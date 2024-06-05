/*
 * Copyright 2024 Fluence DAO
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

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
    #[error("Failed to parse chain data: {0}")]
    ParseChainDataFailedAlloy(#[from] alloy_sol_types::Error),
    #[error("Failed to parse chain data: {0}")]
    RpcChainData(#[from] ErrorObjectOwned),
    #[error("Failed to convert CID to string: {0}")]
    ConvertCid(#[from] libipld::cid::Error),
    #[error("Failed to parse address: {0}")]
    AddressParseError(#[from] clarity::Error),
    #[error("data is not a valid hex: '{0}'")]
    DecodeHex(#[from] hex::FromHexError),
    #[error("empty data is not a valid hex: '{0}'")]
    EmptyData(String),
    #[error(transparent)]
    DecodeConstHex(#[from] const_hex::FromHexError),
    #[error("data is not a valid string: '{0}'")]
    DecodeData(#[from] FromUtf8Error),
    #[error("Failed to parse u256, got: {0}, error: {1}")]
    InvalidU256(String, String),
    #[error("Failed to parse response: {0}")]
    ResponseParseError(String),
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
