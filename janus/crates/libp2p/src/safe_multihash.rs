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

use libp2p::kad::record::Key;
use libp2p::PeerId;
use multihash::Multihash;
use std::borrow::Borrow;
use std::convert::TryFrom;

// Ought to produce same multihash bytes from both "inlining" and "hashing" PeerId
#[derive(Hash, Clone, Debug, PartialEq, Eq)]
pub struct SafeMultihash(Multihash);

impl SafeMultihash {
    pub fn to_base58(&self) -> String {
        bs58::encode(self.0.as_bytes()).into_string()
    }
}

impl From<PeerId> for SafeMultihash {
    fn from(peer_id: PeerId) -> Self {
        let base58 = peer_id.to_base58();
        let bytes = bs58::decode(base58).into_vec().expect("impossible");
        let mhash = Multihash::from_bytes(bytes).expect("impossible");
        SafeMultihash(mhash)
    }
}

impl Into<Key> for SafeMultihash {
    fn into(self) -> Key {
        self.0.into()
    }
}

impl Into<Multihash> for SafeMultihash {
    fn into(self) -> Multihash {
        self.0
    }
}

impl From<Multihash> for SafeMultihash {
    fn from(mhash: Multihash) -> Self {
        SafeMultihash(mhash)
    }
}

impl Borrow<[u8]> for SafeMultihash {
    fn borrow(&self) -> &[u8] {
        self.0.borrow()
    }
}

impl TryFrom<Key> for SafeMultihash {
    type Error = multihash::DecodeOwnedError;

    fn try_from(value: Key) -> Result<Self, Self::Error> {
        Multihash::try_from(value.to_vec()).map(SafeMultihash)
    }
}
