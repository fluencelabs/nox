/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use alloy_sol_types::sol;

sol! {
    struct CIDV1 {
        bytes4 prefixes;
        bytes32 hash;
    }

    event ComputeUnitsMatched(
        bytes32 indexed peerId,
        address deal,
        bytes32 onchainWorkerId,
        bytes32[] cuIds,
        CIDV1 appCID
    );
}

#[cfg(test)]
mod tests {
    use crate::event::compute_unit_matched::{ComputeUnitsMatched, CIDV1};
    use alloy_primitives::Address;
    use alloy_sol_types::SolEvent;
    use chain_data::{parse_log, parse_peer_id, peer_id_to_bytes, Log};
    use fluence_libp2p::RandomPeerId;
    use hex_utils::{decode_hex, encode_hex_0x};
    use libipld::Cid;
    use std::str::FromStr;

    #[test]
    fn topic() {
        assert_eq!(
            ComputeUnitsMatched::SIGNATURE_HASH.to_string(),
            String::from("0x6e5629a2cfaa82d1ea7ad51936794f666271fbd5068017020eaaafdbb017e615")
        );
    }

    #[test]
    fn peer_id() {
        let bytes = [
            88, 198, 255, 218, 126, 170, 188, 84, 84, 39, 255, 137, 18, 55, 7, 139, 121, 207, 149,
            42, 196, 115, 102, 160, 4, 47, 227, 62, 7, 53, 189, 15,
        ];
        let peer_id = parse_peer_id(&bytes).expect("parse peer_id from Token");
        assert_eq!(
            peer_id.to_string(),
            String::from("12D3KooWFnv3Qc25eKpTDCNBoW1jXHMHHHSzcJoPkHai1b2dHNra")
        );

        let hex = "0x7a82a5feefcaad4a89c689412031e5f87c02b29e3fced583be5f05c7077354b7";
        let bytes = decode_hex(hex).expect("parse peer_id from hex");
        let peer_id = parse_peer_id(&bytes).expect("parse peer_id from Token");
        assert_eq!(
            peer_id.to_string(),
            String::from("12D3KooWJ4bTHirdTFNZpCS72TAzwtdmavTBkkEXtzo6wHL25CtE")
        );
    }

    #[test]
    fn parse() {
        let peer_id1 = RandomPeerId::random();
        let cu_id1 = [1u8; 32];
        let deal1 = "0xFfA0611a099AB68AD7C3C67B4cA5bbBEE7a58B99";
        let cid = "bafkreifbi2xutxzrzgohtiyoysxcvozeixmmluqc5jmpvhvjzp7ulvavfy";
        let cid_bytes = Cid::from_str(cid).unwrap().to_bytes();
        let encoded_cid = CIDV1 {
            prefixes: cid_bytes[0..4].as_chunks::<4>().0[0].into(),
            hash: cid_bytes[4..36].as_chunks::<32>().0[0].into(),
        };
        let event1 = ComputeUnitsMatched {
            peerId: peer_id_to_bytes(peer_id1).into(),
            deal: Address::from_slice(&decode_hex(deal1).unwrap()),
            onchainWorkerId: Default::default(),
            cuIds: vec![cu_id1.into()],
            appCID: encoded_cid.clone(),
        };

        let peer_id2 = RandomPeerId::random();
        let cu_id2 = [2u8; 32];
        let cu_id3 = [3u8; 32];
        let deal2 = "0x67b2AD3866429282e16e55B715d12A77F85B7CE8";
        let event2 = ComputeUnitsMatched {
            peerId: peer_id_to_bytes(peer_id2).into(),
            deal: Address::from_slice(&decode_hex(deal2).unwrap()),
            onchainWorkerId: Default::default(),
            cuIds: vec![cu_id2.into(), cu_id3.into()],
            appCID: encoded_cid,
        };
        let log1 = Log {
            data: encode_hex_0x(&event1.encode_data()),
            block_number: "0x0".to_string(),
            removed: false,
            topics: vec![
                ComputeUnitsMatched::SIGNATURE_HASH.to_string(),
                encode_hex_0x(&peer_id_to_bytes(peer_id1)),
            ],
        };
        let log2 = Log {
            data: encode_hex_0x(&event2.encode_data()),
            block_number: "0x1".to_string(),
            removed: false,
            topics: vec![
                ComputeUnitsMatched::SIGNATURE_HASH.to_string(),
                encode_hex_0x(&peer_id_to_bytes(peer_id2)),
            ],
        };

        let m = parse_log::<ComputeUnitsMatched>(log1).expect("error parsing Match from log");
        assert_eq!(parse_peer_id(m.peerId.as_slice()).unwrap(), peer_id1);
        assert_eq!(m.deal.to_string(), deal1);
        assert_eq!(m.cuIds.len(), 1);
        assert_eq!(m.cuIds[0], cu_id1);

        let cid_bytes = [m.appCID.prefixes.to_vec(), m.appCID.hash.to_vec()].concat();
        let app_cid = libipld::Cid::read_bytes(cid_bytes.as_slice())
            .unwrap()
            .to_string();
        assert_eq!(app_cid, cid);

        let m = parse_log::<ComputeUnitsMatched>(log2).expect("error parsing Match from log");
        assert_eq!(parse_peer_id(m.peerId.as_slice()).unwrap(), peer_id2);
        assert_eq!(m.deal.to_string(), deal2);
        assert_eq!(m.cuIds.len(), 2);
        assert_eq!(m.cuIds[0], cu_id2);
        assert_eq!(m.cuIds[1], cu_id3);

        let cid_bytes = [m.appCID.prefixes.to_vec(), m.appCID.hash.to_vec()].concat();
        let app_cid = libipld::Cid::read_bytes(cid_bytes.as_slice())
            .unwrap()
            .to_string();
        assert_eq!(app_cid, cid);
    }
}
