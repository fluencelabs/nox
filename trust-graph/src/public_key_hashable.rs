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

use crate::ed25519::PublicKey;

use ref_cast::RefCast;
use serde::Deserialize;
use std::{
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
};

/// Wrapper to use PublicKey in HashMap
#[derive(PartialEq, Eq, Debug, Clone, RefCast, Deserialize)]
#[repr(transparent)]
pub struct PublicKeyHashable(PublicKey);

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for PublicKeyHashable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0.encode());
        state.finish();
    }

    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        // TODO check for overflow
        let mut bytes: Vec<u8> = Vec::with_capacity(data.len() * 32);
        for d in data {
            bytes.extend_from_slice(&d.0.encode())
        }
        state.write(bytes.as_slice());
        state.finish();
    }
}

impl From<PublicKey> for PublicKeyHashable {
    fn from(pk: PublicKey) -> Self {
        Self(pk)
    }
}

impl Into<PublicKey> for PublicKeyHashable {
    fn into(self) -> PublicKey {
        self.0
    }
}

impl AsRef<PublicKey> for PublicKeyHashable {
    fn as_ref(&self) -> &PublicKey {
        &self.0
    }
}

impl AsRef<PublicKeyHashable> for PublicKey {
    fn as_ref(&self) -> &PublicKeyHashable {
        PublicKeyHashable::ref_cast(self)
    }
}

impl Display for PublicKeyHashable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bs58::encode(self.0.encode()).into_string())
    }
}
