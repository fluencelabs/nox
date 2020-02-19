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

use libp2p::identity::ed25519::{Keypair as Libp2pKeyPair, PublicKey};
use libp2p::identity::error::DecodingError;

pub type Signature = Vec<u8>;

/// An Ed25519 keypair.
#[derive(Clone, Debug)]
pub struct KeyPair {
    pub key_pair: Libp2pKeyPair,
}

impl KeyPair {
    /// Generate a new Ed25519 keypair.
    #[allow(dead_code)]
    pub fn generate() -> KeyPair {
        let kp = Libp2pKeyPair::generate();
        kp.into()
    }

    /// Encode the keypair into a byte array by concatenating the bytes
    /// of the secret scalar and the compressed public point/
    #[allow(dead_code)]
    pub fn encode(&self) -> [u8; 64] {
        self.key_pair.encode()
    }

    /// Decode a keypair from the format produced by `encode`.
    #[allow(dead_code)]
    pub fn decode(kp: &mut [u8]) -> Result<KeyPair, DecodingError> {
        let kp = Libp2pKeyPair::decode(kp)?;
        Ok(Self { key_pair: kp })
    }

    /// Get the public key of this keypair.
    #[allow(dead_code)]
    pub fn show_public_key(&self) -> PublicKey {
        self.key_pair.public()
    }

    /// Sign a message using the private key of this keypair.
    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.key_pair.sign(msg)
    }

    /// Verify the Ed25519 signature on a message using the public key.
    pub fn verify(pk: &PublicKey, msg: &[u8], signature: &[u8]) -> bool {
        pk.verify(msg, signature)
    }
}

impl From<Libp2pKeyPair> for KeyPair {
    fn from(kp: Libp2pKeyPair) -> Self {
        Self { key_pair: kp }
    }
}
