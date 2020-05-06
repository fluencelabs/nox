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

#[derive(Debug)]
pub enum SignatureError {
    MissingClientPublicKey,
    InvalidSignature,
    UnsupportedAddress,
}

fn verify_signature(address: Address, sig: &[u8]) -> Result<(), SignatureError> {
    use Protocol::*;
    use SignatureError::*;

    let protocols = address.protocols();
    let public_key = match protocols.as_slice() {
        [Peer(_), Client(client)] => client.as_public_key().ok_or(MissingClientPublicKey)?,
        _ => {
            log::warn!(
                "Signature verification error {}: not a relay address",
                address
            );
            return Err(UnsupportedAddress);
        }
    };

    let path = address.path().as_bytes();
    if !public_key.verify(path, sig) {
        return Err(InvalidSignature);
    }

    Ok(())
}

// Search through protocols in address, look for signatures, verify signature against all protocols
// behind signature.
// NOTE: will return `Ok` if there are no signatures, so it is caller responsibility to handle signature absence.
/// Example 1:
///     For `[Peer(QmPeer), Client(QmClient), Signature(sig), Peer(QmPeer2), Client(QmClient2), Signature(sig2)]`
///     will check both signatures, for `/peer/QmPeer/client/QmClient` and `/peer/QmPeer2/client/QmClient2` addresses
/// Example 2:
///     For `[Peer(QmPeer), Client(QmClient), Peer(QmPeer2), Signature(sig)]` will create address
///     from first 3 elements: `"/peer/QmPeer/client/QmClient/peer/QmPeer2"`, convert to bytes,
///     and verify signature against these bytes.
/// NOTE: However, only relay addresses are supported as of 0.0.5, so Example 2 will actually
/// return an error, so it is only for demonstration purposes.
///
pub fn verify_address(address: &Address) -> Result<(), SignatureError> {
    address.iter().fold(Ok(Address::empty()), |addr, proto| {
        if let Protocol::Signature(sig) = proto {
            verify_signature(addr.unwrap(), &sig)?;
            Ok(Address::empty())
        } else {
            addr.map(|addr| addr.append(proto))
        }
    })?;

    Ok(())
}

// Filter out all Protocol::Signature protocols, and rebuilds address without them
pub fn remove_signatures(address: Address) -> Address {
    address
        .iter()
        .filter(|p| !matches!(p, Protocol::Signature(_)))
        .collect()
}
