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

use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration;

use derivative::Derivative;
use fluence_keypair::{KeyPair, PublicKey, Signature};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use tracing::Span;

use crate::error::ParticleError;
use crate::error::ParticleError::{
    DecodingError, InvalidKeypair, SignatureVerificationFailed, SigningFailed,
};
use fluence_libp2p::{peerid_serializer, RandomPeerId};
use json_utils::base64_serde;
use now_millis::now_ms;
#[derive(Clone, Debug)]
pub struct ExtendedParticle {
    pub particle: Particle,
    pub span: Arc<Span>,
}


#[derive(Clone, Serialize, Deserialize, PartialEq, Derivative)]
#[derivative(Debug)]
pub struct Particle {
    pub id: String,
    #[serde(with = "peerid_serializer")]
    pub init_peer_id: PeerId,
    // Unix timestamp in milliseconds
    pub timestamp: u64,
    // TTL in milliseconds
    pub ttl: u32,
    pub script: String,
    pub signature: Vec<u8>,
    /// base64-encoded
    #[serde(with = "base64_serde")]
    #[derivative(Debug(format_with = "fmt_data"))]
    pub data: Vec<u8>,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            // TODO: sure random peer id is OK as default?
            init_peer_id: RandomPeerId::random(),
            timestamp: 0,
            ttl: 0,
            script: "".to_string(),
            signature: vec![],
            data: vec![],
        }
    }
}

impl Particle {
    pub fn is_expired(&self) -> bool {
        if let Some(deadline) = self.deadline() {
            return now_ms() > deadline as u128;
        }

        // If timestamp + ttl overflows u64, consider particle expired
        true
    }

    /// Deadline in milliseconds
    #[inline]
    pub fn deadline(&self) -> Option<u64> {
        self.timestamp.checked_add(self.ttl as u64)
    }

    pub fn time_to_live(&self) -> Duration {
        if let Some(ttl) = self.deadline().and_then(|d| d.checked_sub(now_ms() as u64)) {
            Duration::from_millis(ttl)
        } else {
            Duration::default()
        }
    }

    /// return immutable particle fields in bytes for signing
    /// concatenation of:
    /// - id as bytes
    /// - timestamp u64 as little-endian bytes
    /// - ttl u32 as little-endian bytes
    /// - script as bytes
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend(self.id.as_bytes());
        bytes.extend(self.timestamp.to_le_bytes());
        bytes.extend(self.ttl.to_le_bytes());
        bytes.extend(self.script.as_bytes());

        bytes
    }

    pub fn sign(&mut self, keypair: &KeyPair) -> Result<(), ParticleError> {
        if self.init_peer_id != keypair.get_peer_id() {
            return Err(InvalidKeypair {
                particle_id: self.id.clone(),
                init_peer_id: self.init_peer_id.to_base58(),
                given_peer_id: keypair.get_peer_id().to_base58(),
            });
        }
        self.signature = keypair
            .sign(self.as_bytes().as_slice())
            .map_err(|err| SigningFailed {
                err,
                particle_id: self.id.clone(),
            })?
            .to_vec()
            .to_vec();

        Ok(())
    }

    pub fn verify(&self) -> Result<(), ParticleError> {
        let pk: PublicKey = self.init_peer_id.try_into().map_err(|err| DecodingError {
            err,
            particle_id: self.id.clone(),
        })?;
        let sig = Signature::from_bytes(pk.get_key_format(), self.signature.clone());
        pk.verify(&self.as_bytes(), &sig)
            .map_err(|err| SignatureVerificationFailed {
                err,
                particle_id: self.id.clone(),
                peer_id: self.init_peer_id.to_base58(),
            })
    }
}

#[allow(clippy::ptr_arg)]
fn fmt_data(data: &Vec<u8>, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    use base64::{engine::general_purpose::STANDARD as base64, Engine};

    write!(f, "{}", base64.encode(data))
}

impl std::fmt::Display for Particle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "particle id {}, init_user_id {}, timestamp {}, ttl {}, data [{} bytes]",
            self.id,
            self.init_peer_id,
            self.timestamp,
            self.ttl,
            self.data.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::Particle;
    use base64::{engine::general_purpose::STANDARD as base64, Engine};
    use fluence_keypair::{KeyFormat, KeyPair};

    #[test]
    fn test_signature() {
        let kp_bytes = base64
            .decode("7h48PQ/f1rS9TxacmgODxbD42Il9B3KC117jvOPppPE=")
            .unwrap();
        assert_eq!(kp_bytes.len(), 32);

        let kp = KeyPair::from_secret_key(kp_bytes, KeyFormat::Ed25519).unwrap();

        // assert peer id
        assert_eq!(
            kp.get_peer_id().to_base58(),
            "12D3KooWANqfCDrV79MZdMnMqTvDdqSAPSxdgFY1L6DCq2DVGB4D"
        );

        // test simple signature
        let message = "message".to_string();

        let signature = kp.sign(message.as_bytes()).unwrap();
        assert!(kp.public().verify(message.as_bytes(), &signature).is_ok());
        assert_eq!(base64.encode(signature.to_vec()), "sBW7H6/1fwAwF86ldwVm9BDu0YH3w30oFQjTWX0Tiu9yTVZHmxkV2OX4GL5jn0Iz0CrasGcOfozzkZwtJBPMBg==");

        // test particle signature
        let mut p = Particle {
            id: "2883f959-e9e7-4843-8c37-205d393ca372".to_string(),
            init_peer_id: kp.get_peer_id(),
            timestamp: 1696934545662,
            ttl: 7000,
            script: "abc".to_string(),
            signature: vec![],
            data: vec![],
        };

        let particle_bytes = p.as_bytes();
        assert_eq!(
            base64.encode(&particle_bytes),
            "Mjg4M2Y5NTktZTllNy00ODQzLThjMzctMjA1ZDM5M2NhMzcy/kguGYsBAABYGwAAYWJj"
        );

        p.sign(&kp).unwrap();
        assert!(p.verify().is_ok());
        assert_eq!(base64.encode(&p.signature), "KceXDnOfqe0dOnAxiDsyWBIvUq6WHoT0ge+VMHXOZsjZvCNH7/10oufdlYfcPomfv28On6E87ZhDcHGBZcb7Bw==");
    }
}
