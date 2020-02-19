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

use crate::trust::key_pair::Signature;
use libp2p::identity::ed25519::PublicKey;
use std::convert::TryInto;
use std::time::Duration;

const SIGNATURE_LEN: usize = 64;
const PUBLICKEY_LEN: usize = 32;
const EXPIRATION_LEN: usize = 8;
pub const TRUST_LEN: usize = SIGNATURE_LEN + PUBLICKEY_LEN + EXPIRATION_LEN;

/// One element in chain of trust in a certificate.
#[derive(Clone, Debug, PartialEq)]
pub struct Trust {
    pub pk: PublicKey,
    /// Expiration date of a trust.
    pub expires_at: Duration,
    /// Signature of a previous trust in a chain.
    /// Signature is self-signed if it is a root trust.
    pub signature: Signature,
}

impl Trust {
    #[allow(dead_code)]
    pub fn encode(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(TRUST_LEN);
        vec.extend_from_slice(&self.pk.encode());
        vec.extend_from_slice(&self.signature.as_slice());
        vec.extend_from_slice(&(self.expires_at.as_millis() as u64).to_le_bytes());

        vec
    }

    #[allow(dead_code)]
    pub fn decode(arr: &[u8]) -> Result<Trust, String> {
        if arr.len() != TRUST_LEN {
            return Err(
                "Trust length should be 104: public key(32) + signature(64) + expiration date(8)"
                    .to_string(),
            );
        }

        let pk = PublicKey::decode(&arr[0..PUBLICKEY_LEN]).map_err(|err| err.to_string())?;
        let signature = &arr[PUBLICKEY_LEN..PUBLICKEY_LEN + SIGNATURE_LEN];

        let expiration_bytes = &arr[PUBLICKEY_LEN + SIGNATURE_LEN..TRUST_LEN];
        let expiration_date = u64::from_le_bytes(expiration_bytes.try_into().unwrap());
        let expiration_date = Duration::from_millis(expiration_date);

        Ok(Self {
            pk,
            signature: signature.to_vec(),
            expires_at: expiration_date,
        })
    }
}
