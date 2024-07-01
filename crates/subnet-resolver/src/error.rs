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
