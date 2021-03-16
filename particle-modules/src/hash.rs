/*
 * Copyright 2020 Fluence Labs Limited
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

use blake3::hash;
use faster_hex::hex_decode;
use serde::export::Formatter;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hash(blake3::Hash);
impl Hash {
    /// Construct Hash from raw hash value in hex; doesn't hash anything
    pub fn from_hex(hash: &str) -> Result<Self, faster_hex::Error> {
        let mut buf: [u8; blake3::OUT_LEN] = [0; blake3::OUT_LEN];
        hex_decode(hash.as_bytes(), &mut buf)?;
        Ok(Self::from(buf))
    }

    /// Hash arbitrary bytes
    ///
    /// see `From<[u8; blake3::OUT_LEN]>` to create from raw bytes without hashing them
    pub fn hash(bytes: &[u8]) -> Self {
        Self(hash(bytes))
    }

    /// Converts hash to hex str
    pub fn to_hex(&self) -> impl AsRef<str> {
        self.0.to_hex()
    }

    pub fn as_bytes(&self) -> &[u8; blake3::OUT_LEN] {
        self.0.as_bytes()
    }
}

/// Creates a hash from raw bytes without hashing them
impl From<[u8; blake3::OUT_LEN]> for Hash {
    #[inline]
    fn from(bytes: [u8; blake3::OUT_LEN]) -> Self {
        Self(blake3::Hash::from(bytes))
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.to_hex().as_ref().fmt(f)
    }
}
