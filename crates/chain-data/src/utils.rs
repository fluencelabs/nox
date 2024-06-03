/*
 * Copyright 2024 Fluence DAO
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

use hex_utils::decode_hex;
use libp2p_identity::{ParseError, PeerId};

/// Static prefix of the PeerId. Protobuf encoding + multihash::identity + length and so on.
pub(crate) const PEER_ID_PREFIX: &[u8] = &[0, 36, 8, 1, 18, 32];

pub fn parse_peer_id(bytes: Vec<u8>) -> Result<PeerId, ParseError> {
    let peer_id = [PEER_ID_PREFIX, &bytes].concat();

    PeerId::from_bytes(&peer_id)
}

/// This code works only for PeerId generated from ed25519 public key, the size assumptions is wrong
pub fn peer_id_to_bytes(peer_id: PeerId) -> [u8; 32] {
    let peer_id = peer_id.to_bytes();
    // peer_id is 38 bytes but we need 32 for chain
    let res = peer_id[PEER_ID_PREFIX.len()..].as_chunks::<32>();
    res.0[0]
}
pub fn peer_id_to_hex(peer_id: PeerId) -> String {
    format!("0x{:0>64}", hex::encode(peer_id_to_bytes(peer_id)))
}

pub fn peer_id_from_hex(hex: &str) -> eyre::Result<PeerId> {
    Ok(parse_peer_id(decode_hex(hex)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn peer_id_test() {
        let hex = "0x246cd65bc58db104674f76c9b1340eb16881d9ef90e33d4b1086ebd334f4002d".to_string();
        let peer_id =
            PeerId::from_str("12D3KooWCGZ6t8by5ag5YMQW4k3HoPLaKdN5rB9DhAmDUeG8dj1N").unwrap();
        assert_eq!(
            peer_id,
            parse_peer_id(hex::decode(&hex[2..]).unwrap()).unwrap()
        );
        assert_eq!(hex, peer_id_to_hex(peer_id));
    }
}
