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

use faas_api::{Address, Protocol};
use libp2p::identity::PublicKey;
use libp2p::PeerId;

#[derive(Debug)]
pub enum SignatureError {
    MissingClientPublicKey,
    InvalidSignature,
    UnsupportedAddress,
    MissingSignature,
}

pub fn extract_client_id(address: &Address) -> Result<PeerId, SignatureError> {
    use Protocol::*;
    use SignatureError::*;

    let mut ps = address.iter();
    match (ps.next(), ps.next(), ps.next()) {
        (Some(Peer(_)), Some(Client(client)), None) => Ok(client),
        _ => {
            log::warn!(
                "Signature verification error {}: not a relay address",
                address
            );
            Err(UnsupportedAddress)
        }
    }
}

/// Extract client public key from relay address,
/// return error if it's not a relay address, or
/// there's no public key in client's PeerId
pub fn extract_public_key(address: &Address) -> Result<PublicKey, SignatureError> {
    extract_client_id(address)?
        .as_public_key()
        .ok_or(SignatureError::MissingClientPublicKey)
}

/// Verify signature of the address path (without schema)
pub fn verify_signature(addr: &Address, sig: &[u8], pk: &PublicKey) -> Result<(), SignatureError> {
    use SignatureError::*;

    let path = addr.path().as_bytes();
    if !pk.verify(path, sig) {
        return Err(InvalidSignature);
    }

    Ok(())
}

/// Search through protocols in address, look for signatures,
/// verify signature against all protocols behind signature.
///
/// NOTE: will return `Ok` if there are no signatures, so it is
/// a caller responsibility to handle signature absence.
///
/// Example 1:
///     For `/peer/QmPeer/client/QmClient/signature/sig/peer/QmPeer2/client/QmClient2/signature/sig2`
///     will check both signatures, for `/peer/QmPeer/client/QmClient` and `/peer/QmPeer2/client/QmClient2` addresses
/// Example 2:
///     For `/peer/QmPeer/client/QmClient/peer/QmPeer2/signature/sig` will create address
///     from first 3 elements: `"/peer/QmPeer/client/QmClient/peer/QmPeer2"`, convert to bytes,
///     and verify signature against these bytes.
///
/// EXAMPLE NOTE: However, since only relay addresses are supported as of 0.0.5,
/// Example 2 will actually return an error, so it is only for demonstration purposes.
///
pub fn verify_address_signatures(address: &Address) -> Result<(), SignatureError> {
    address.iter().fold(Ok(Address::empty()), |addr, proto| {
        if let Protocol::Signature(sig) = proto {
            let address = addr.unwrap();
            let pk = extract_public_key(&address)?;
            verify_signature(&address, &sig, &pk)?;
            Ok(Address::empty())
        } else {
            addr.map(|addr| addr.append(proto))
        }
    })?;

    Ok(())
}
