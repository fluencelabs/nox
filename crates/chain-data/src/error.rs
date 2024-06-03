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
