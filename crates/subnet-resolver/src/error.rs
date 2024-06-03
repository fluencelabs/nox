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
