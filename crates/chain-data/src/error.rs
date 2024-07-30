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

use thiserror::Error;

use hex_utils::FromHexError;

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
    DecodeHex(#[source] FromHexError),
    #[error(transparent)]
    EthError(#[from] ethabi::Error),
    #[error("Invalid token size")]
    InvalidTokenSize,
}
