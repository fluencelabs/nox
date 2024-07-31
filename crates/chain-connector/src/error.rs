/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::string::FromUtf8Error;

use jsonrpsee::core::client::{Error as RPCError, Error};
use jsonrpsee::types::ErrorObjectOwned;
use thiserror::Error;

use chain_data::ChainDataError;
use hex_utils::FromHexError;

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
    DecodeHex(#[from] FromHexError),
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
    #[error("Failed to parse response, field {0} not found")]
    FieldNotFound(&'static str),
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
