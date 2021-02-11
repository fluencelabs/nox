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

use crate::file_names::{module_config_name, module_file_name};
use blake3::hash;
use serde::export::Formatter;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt::Display;

/// Hash of the .wasm module contents
///
/// Used to load & store modules and configs on filesystem
#[derive(Debug, Clone)]
pub struct ModuleHash(blake3::Hash);
impl ModuleHash {
    /// Hash arbitrary bytes
    ///
    /// see `From<[u8; blake3::OUT_LEN]>` to create from raw bytes without hashing them
    pub fn hash(bytes: &[u8]) -> Self {
        Self(hash(bytes))
    }

    /// Converts module hash to hex str
    pub fn to_hex(&self) -> impl AsRef<str> {
        self.0.to_hex()
    }

    /// Returns file name for configuration file of the module
    pub fn config_file_name(&self) -> String {
        module_config_name(&self)
    }

    /// Returns file name of the wasm module
    pub fn wasm_file_name(&self) -> String {
        module_file_name(&self)
    }
}

/// Creates ModuleHash from raw bytes without hashing them
impl From<[u8; blake3::OUT_LEN]> for ModuleHash {
    #[inline]
    fn from(bytes: [u8; blake3::OUT_LEN]) -> Self {
        Self(blake3::Hash::from(bytes))
    }
}

impl Display for ModuleHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.to_hex().as_ref().fmt(f)
    }
}

#[derive(Debug, Clone)]
pub enum Dependency {
    Hash(ModuleHash),
    Name(String),
}

impl Display for Dependency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Dependency::Hash(hash) => hash.fmt(f),
            Dependency::Name(name) => name.fmt(f),
        }
    }
}

impl<'de> Deserialize<'de> for Dependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <Cow<'de, str>>::deserialize(deserializer)?;
        let mut s = s.split(":");
        let id_val: Option<(_, _)> = try {
            let id = s.next()?;
            let value = s.next();
            (id, value)
        };
        let id_val = id_val.ok_or(de::Error::missing_field("dependency"))?;

        let value = match id_val {
            ("hash", Some(hash)) => Dependency::Hash(ModuleHash::from(from_hex(hash))),
            ("name", Some(name)) | (name, _) => Dependency::Name(name.to_string()),
        };

        Ok(value)
    }
}

impl Serialize for Dependency {
    fn serialize<S>(&self, s: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        match self {
            Dependency::Hash(h) => h.to_hex().as_ref().serialize(s),
            Dependency::Name(n) => n.serialize(s),
        }
    }
}

fn from_hex(s: &str) -> [u8; 32] {
    let mut out: [u8; 32] = [0; 32];
    let bs = s.as_bytes();
    for i in 0..32usize {
        let c = bs[i];
        let value = (c & 0x0f) + 9 * (c >> 6);
        out[i] = value;
    }

    out
}

mod tests {
    #[test]
    fn test() {}
}
