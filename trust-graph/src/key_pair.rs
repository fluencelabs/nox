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

use crate::ed25519::{Keypair as Libp2pKeyPair, PublicKey, SecretKey};
use ed25519_dalek::SignatureError;
use libp2p_core::identity::error::DecodingError;
use std::fmt;

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
    pub fn decode(kp: &[u8]) -> Result<KeyPair, SignatureError> {
        let kp = ed25519_dalek::Keypair::from_bytes(kp)?;
        Ok(Self {
            key_pair: kp.into(),
        })
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

/// Implement serde::Deserialize for KeyPair
impl<'de> serde::Deserialize<'de> for KeyPair {
    fn deserialize<D>(deserializer: D) -> Result<KeyPair, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Unexpected, Visitor};

        struct KeyPairVisitor;

        impl<'de> Visitor<'de> for KeyPairVisitor {
            type Value = KeyPair;

            /// Error message stating what was expected
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("byte array or base58 string")
            }

            /// Implement deserialization from base58 string
            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                bs58::decode(s)
                    .into_vec()
                    .map_err(|_| Error::invalid_value(Unexpected::Str(s), &self))
                    .and_then(|v| self.visit_bytes(v.as_slice()))
            }

            /// Implement deserialization from bytes
            fn visit_bytes<E>(self, b: &[u8]) -> Result<Self::Value, E>
            where
                E: Error,
            {
                KeyPair::decode(b).map_err(|_| Error::invalid_value(Unexpected::Bytes(b), &self))
            }
        }

        deserializer.deserialize_str(KeyPairVisitor)
    }
}
