/*
 * Copyright 2018 Fluence Labs Limited
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

use ethabi::Token;
use web3::contract::tokens::Tokenizable;
use web3::contract::{Error as ContractError, ErrorKind};

/// number of bytes for encoding an IP address
pub const IP_LEN: usize = 4;

/// number of bytes for encoding tendermint key
pub const TENDERMINT_KEY_LEN: usize = 20;

/// number of bytes for encoding IP address and tendermint key
pub const NODE_ADDR_LEN: usize = IP_LEN + TENDERMINT_KEY_LEN;
construct_fixed_hash!{ pub struct H192(NODE_ADDR_LEN); }

/// Helper for converting the hash structure to web3 format
impl Tokenizable for H192 {
    fn from_token(token: Token) -> Result<Self, ContractError> {
        match token {
            Token::FixedBytes(mut s) => {
                if s.len() != NODE_ADDR_LEN {
                    bail!(ErrorKind::InvalidOutputType(format!(
                        "Expected `H192`, got {:?}",
                        s
                    )));
                }
                let mut data = [0; NODE_ADDR_LEN];
                for (idx, val) in s.drain(..).enumerate() {
                    data[idx] = val;
                }
                Ok(data.into())
            }
            other => Err(
                ErrorKind::InvalidOutputType(format!("Expected `H192`, got {:?}", other)).into(),
            ),
        }
    }

    fn into_token(self) -> Token {
        Token::FixedBytes(self.0.to_vec())
    }
}
