/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use crate::ed25519::PublicKey;
use std::fmt::Display;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};

/// Wrapper to use PublicKey in HashMap
#[derive(PartialEq, Eq, Debug, Clone)]
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

impl Display for PublicKeyHashable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bs58::encode(self.0.encode()).into_string())
    }
}
