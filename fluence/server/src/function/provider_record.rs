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

use crate::function::address_signature::{
    verify_address_signatures, verify_signature, SignatureError,
};
use faas_api::{Address, AddressError};
use libp2p::identity::ed25519::Keypair;
use libp2p::PeerId;
use prost::Message;

#[derive(Debug)]
pub enum ProviderError {
    Deserialization(AddressError),
    Signature(SignatureError),
    NoPublisherKey,
}

// This will be stored in DHT in libp2p::kad::record::Record::value
// Name (i.e., service_id or client's peer_id) isn't stored here, because it is stored as a key in DHT
// This structure meant to be serialized via protobuf, and stored in DHT as bytes
#[derive(Clone, PartialEq, Message)]
#[prost(tags = "sequential")]
pub struct ProviderRecord {
    #[prost(string, required)]
    // Where to send requests that are targeted to the provider
    pub address: String,
    #[prost(bytes, required)]
    // Signature of the address path (i.e., without schema) by author of the DHT record (usually: relay peer)
    pub signature: Vec<u8>,
}

impl ProviderRecord {
    // Create ProviderRecord, signing address path (without schema) with passed keypair
    pub fn new(address: Address, kp: &Keypair) -> Self {
        let address = address.path();
        let signature = kp.sign(address.as_bytes());

        Self {
            address: address.into(),
            signature,
        }
    }

    // Deserialize value to ProviderRecord, verify address and record author's signature.
    // Return provider Address.
    pub fn deserialize_address(value: &[u8], publisher: &PeerId) -> Result<Address, ProviderError> {
        use ProviderError::*;

        let provider: ProviderRecord = value.into();
        let address: Address = provider.address.parse().map_err(|e| Deserialization(e))?;
        let public = publisher.as_public_key().ok_or(NoPublisherKey)?;
        let sig = provider.signature.as_slice();

        // Verify record author signature
        verify_signature(&address, sig, &public).map_err(|e| Signature(e))?;
        // Verify client signatures
        verify_address_signatures(&address).map_err(|e| Signature(e))?;

        Ok(address)
    }
}

impl Into<Vec<u8>> for ProviderRecord {
    // Encode ProviderRecord to bytes with protobuf
    fn into(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf).expect("encode ProviderRecord");

        buf
    }
}

impl From<&[u8]> for ProviderRecord {
    // Encode ProviderRecord from bytes with protobuf
    fn from(bytes: &[u8]) -> Self {
        ProviderRecord::decode(bytes).expect("decode ProviderRecord")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::provider_record::ProviderRecord;
    use faas_api::relay;
    use libp2p::PeerId;

    #[test]
    // Serialize, deserialize, check signature
    fn ser_de() {
        let keypair = Keypair::generate();

        let client = PeerId::random();
        let relay = PeerId::random();
        let address: Address = relay!(relay, client);

        let rec = ProviderRecord::new(address.clone(), &keypair);
        let encoded: Vec<u8> = rec.clone().into();
        let decoded: ProviderRecord = encoded.as_slice().into();

        assert_eq!(rec, decoded);
        assert!(
            keypair
                .public()
                .verify(address.path().as_bytes(), &rec.signature),
            "invalid signature"
        );
    }
}
