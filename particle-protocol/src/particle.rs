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
use std::time::Duration;

use derivative::Derivative;
use fluence_keypair::{KeyPair, PublicKey, Signature};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

use fluence_libp2p::{peerid_serializer, RandomPeerId};
use json_utils::base64_serde;
use now_millis::now_ms;

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

    fn as_bytes(&self) -> eyre::Result<Vec<u8>> {
        #[derive(Serialize)]
        struct ParticleMetadata {
            pub id: String,
            #[serde(with = "peerid_serializer")]
            pub init_peer_id: PeerId,
            pub timestamp: u64,
            pub ttl: u32,
            pub script: String,
        }

        Ok(bincode::serialize(&ParticleMetadata {
            id: self.id.clone(),
            init_peer_id: self.init_peer_id,
            timestamp: self.timestamp,
            ttl: self.ttl,
            script: self.script.clone(),
        })?)
    }

    pub fn sign(&mut self, keypair: &KeyPair) -> eyre::Result<()> {
        if self.init_peer_id != keypair.get_peer_id() {
            return Err(eyre::eyre!(
                "Failed to sign particle {} by different peer id: expected {}, found {}",
                self.id,
                self.init_peer_id,
                keypair.get_peer_id()
            ));
        }

        self.signature = keypair.sign(self.as_bytes()?.as_slice())?.to_vec().to_vec();

        Ok(())
    }

    pub fn verify(&self) -> eyre::Result<()> {
        let pk: PublicKey = self.init_peer_id.try_into()?;
        let sig = Signature::from_bytes(pk.get_key_format(), self.signature.clone());
        Ok(pk.verify(&self.as_bytes()?, &sig)?)
    }
}

#[allow(clippy::ptr_arg)]
fn fmt_data(data: &Vec<u8>, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    write!(f, "{}", base64::encode(data))
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
