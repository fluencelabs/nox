use hex_utils::decode_hex;
use libp2p_identity::{ParseError, PeerId};

/// Static prefix of the PeerId. Protobuf encoding + multihash::identity + length and so on.
pub(crate) const PEER_ID_PREFIX: &[u8] = &[0, 36, 8, 1, 18, 32];

pub fn parse_peer_id(bytes: Vec<u8>) -> Result<PeerId, ParseError> {
    let peer_id = [PEER_ID_PREFIX, &bytes].concat();

    PeerId::from_bytes(&peer_id)
}

pub fn peer_id_to_bytes(peer_id: PeerId) -> Vec<u8> {
    let peer_id = peer_id.to_bytes();
    peer_id[PEER_ID_PREFIX.len()..].to_vec()
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
