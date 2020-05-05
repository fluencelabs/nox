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

use crate::ed25519::{Keypair as Libp2pKeyPair, PublicKey, SecretKey};
use libp2p_core::identity::error::DecodingError;

pub type Signature = Vec<u8>;

/// An Ed25519 keypair.
#[derive(Clone, Debug)]
pub struct KeyPair {
    pub key_pair: Libp2pKeyPair,
}

impl KeyPair {
    /// Generate a new Ed25519 keypair.
    #[allow(dead_code)]
    pub fn generate() -> Self {
        let kp = Libp2pKeyPair::generate();
        kp.into()
    }

    pub fn from_bytes(sk_bytes: impl AsMut<[u8]>) -> Result<Self, DecodingError> {
        let sk = SecretKey::from_bytes(sk_bytes)?;
        Ok(Libp2pKeyPair::from(sk).into())
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
    pub fn public_key(&self) -> PublicKey {
        self.key_pair.public()
    }

    /// Sign a message using the private key of this keypair.
    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.key_pair.sign(msg)
    }

    /// Verify the Ed25519 signature on a message using the public key.
    pub fn verify(pk: &PublicKey, msg: &[u8], signature: &[u8]) -> Result<(), String> {
        if pk.verify(msg, signature) {
            return Ok(());
        }

        Err("Signature is not valid.".to_string())
    }
}

impl From<Libp2pKeyPair> for KeyPair {
    fn from(kp: Libp2pKeyPair) -> Self {
        Self { key_pair: kp }
    }
}
