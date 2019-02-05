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

use failure::Error;

use error_chain::bail;
use ethabi::Token;
use ethereum_types_serialize::{deserialize_check_len, serialize};
use fixed_hash::construct_fixed_hash;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::net::IpAddr;
use web3::contract::tokens::Tokenizable;
use web3::contract::{Error as ContractError, ErrorKind};

/// number of bytes for encoding an IP address
pub const IP_LEN: usize = 4;

/// number of bytes for encoding tendermint node id (first 20-bytes of SHA from p2p pub key)
pub const TENDERMINT_NODE_ID_LEN: usize = 20;

/// number of bytes for encoding IP address and tendermint key
pub const NODE_ADDR_LEN: usize = IP_LEN + TENDERMINT_NODE_ID_LEN;
construct_fixed_hash! { pub struct NodeAddress(NODE_ADDR_LEN); }

impl NodeAddress {
    pub fn decode(&self) -> Result<(String, IpAddr), Error> {
        let tendermint_key = &self.0[0..TENDERMINT_NODE_ID_LEN];
        let tendermint_key = format!("{}{}", "0x", hex::encode(tendermint_key));

        let ip_addr = &self.0[TENDERMINT_NODE_ID_LEN..TENDERMINT_NODE_ID_LEN + IP_LEN];
        let ip_addr = format!(
            "{}.{}.{}.{}",
            ip_addr[0], ip_addr[1], ip_addr[2], ip_addr[3]
        );
        let ip_addr: IpAddr = ip_addr.parse()?;

        Ok((tendermint_key, ip_addr))
    }
}

/// Helper for converting the hash structure to web3 format
impl Tokenizable for NodeAddress {
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

impl Serialize for NodeAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut slice = [0u8; 2 + 2 * NODE_ADDR_LEN];
        serialize(&mut slice, &self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for NodeAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut bytes = [0u8; NODE_ADDR_LEN];
        deserialize_check_len(
            deserializer,
            ethereum_types_serialize::ExpectedLen::Exact(&mut bytes),
        )?;
        Ok(NodeAddress(bytes))
    }
}
