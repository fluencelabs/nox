/*
 * Copyright 2019 Fluence Labs Limited
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

use libp2p::kad::record::Key;
use libp2p::PeerId;
use multihash::Multihash;
use std::convert::TryFrom;

// Ought to produce same multihash bytes from both "inlining" and "hashing" PeerId
#[derive(Hash, Clone, Debug, PartialEq, Eq)]
pub struct SafeMultihash(Multihash);
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

impl TryFrom<Key> for SafeMultihash {
    type Error = multihash::DecodeOwnedError;

    fn try_from(value: Key) -> Result<Self, Self::Error> {
        Multihash::try_from(value.to_vec()).map(SafeMultihash)
    }
}
