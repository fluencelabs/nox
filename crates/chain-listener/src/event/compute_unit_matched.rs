use alloy_sol_types::sol;

sol! {
    struct CIDV1 {
        bytes4 prefixes;
        bytes32 hash;
    }

    event ComputeUnitMatched(
        bytes32 indexed peerId,
        address deal,
        bytes32 unitId,
        uint256 dealCreationBlock,
        CIDV1 appCID
    );
}

#[cfg(test)]
mod tests {
    use crate::event::compute_unit_matched::ComputeUnitMatched;
    use alloy_primitives::Uint;
    use alloy_sol_types::SolEvent;
    use chain_data::{parse_log, parse_peer_id, Log};
    use hex_utils::decode_hex;

    #[test]
    fn topic() {
        assert_eq!(
            ComputeUnitMatched::SIGNATURE_HASH.to_string(),
            String::from("0xb1c5a9179c3104a43de668491f14c45778f00ec34d5deee023af204820483bdb")
        );
    }

    #[test]
    fn peer_id() {
        let bytes = [
            88, 198, 255, 218, 126, 170, 188, 84, 84, 39, 255, 137, 18, 55, 7, 139, 121, 207, 149,
            42, 196, 115, 102, 160, 4, 47, 227, 62, 7, 53, 189, 15,
        ];
        let peer_id = parse_peer_id(bytes.into()).expect("parse peer_id from Token");
        assert_eq!(
            peer_id.to_string(),
            String::from("12D3KooWFnv3Qc25eKpTDCNBoW1jXHMHHHSzcJoPkHai1b2dHNra")
        );

        let hex = "0x7a82a5feefcaad4a89c689412031e5f87c02b29e3fced583be5f05c7077354b7";
        let bytes = decode_hex(hex).expect("parse peer_id from hex");
        let peer_id = parse_peer_id(bytes).expect("parse peer_id from Token");
        assert_eq!(
            peer_id.to_string(),
            String::from("12D3KooWJ4bTHirdTFNZpCS72TAzwtdmavTBkkEXtzo6wHL25CtE")
        );
    }

    #[test]
    fn parse() {
        let data1 = "000000000000000000000000ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b9900000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000506a866cfa000000000000000000000000000000000000000000000000000000005a5a0f4fa4d41a4f976e799895cce944d5080041dba7d528d30e81c67973bac3".to_string();
        let data2 = "00000000000000000000000067b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000566a866cfa000000000000000000000000000000000000000000000000000000005a5a0f4fa4d41a4f976e799895cce944d5080041dba7d528d30e81c67973bac3".to_string();
        let log1 = Log {
            data: data1,
            block_number: "0x0".to_string(),
            removed: false,
            topics: vec![
                ComputeUnitMatched::SIGNATURE_HASH.to_string(),
                "0x7a82a5feefcaad4a89c689412031e5f87c02b29e3fced583be5f05c7077354b7".to_string(),
            ],
        };
        let log2 = Log {
            data: data2,
            block_number: "0x1".to_string(),
            removed: false,
            topics: vec![
                ComputeUnitMatched::SIGNATURE_HASH.to_string(),
                "0x7a82a5feefcaad4a89c689412031e5f87c02b29e3fced583be5f05c7077354b7".to_string(),
            ],
        };

        let m = parse_log::<ComputeUnitMatched>(log1).expect("error parsing Match from log");
        assert_eq!(
            parse_peer_id(m.peerId.to_vec()).unwrap().to_string(),
            "12D3KooWJ4bTHirdTFNZpCS72TAzwtdmavTBkkEXtzo6wHL25CtE"
        );
        assert_eq!(
            m.deal.to_string(),
            "0xFfA0611a099AB68AD7C3C67B4cA5bbBEE7a58B99"
        );
        assert_eq!(
            m.unitId.to_string(),
            "0x00000000000000000000000000000000000000000000000000000000000000a0"
        );
        assert_eq!(m.dealCreationBlock, Uint::from(80));

        // let cid_bytes = [m.appCID.prefixes.0.to_vec(), m.appCID.hash.0.to_vec()].concat();
        // assert_eq!("0x6a866cfa", hex::encode(&cid_bytes));
        // let app_cid = libipld::Cid::read_bytes(cid_bytes.as_slice())
        //     .unwrap()
        //     .to_string();
        // assert_eq!(
        //     app_cid,
        //     "bafkreifolrizgmusl4y7or5e5xmvr623a6i3ca4d5rwv457cezhschqj4m"
        // );

        let m = parse_log::<ComputeUnitMatched>(log2).expect("error parsing Match from log");
        assert_eq!(
            parse_peer_id(m.peerId.to_vec()).unwrap().to_string(),
            "12D3KooWJ4bTHirdTFNZpCS72TAzwtdmavTBkkEXtzo6wHL25CtE"
        );
        assert_eq!(
            m.deal.to_string(),
            "0x67b2AD3866429282e16e55B715d12A77F85B7CE8"
        );
        assert_eq!(
            m.unitId.to_string(),
            "0x00000000000000000000000000000000000000000000000000000000000000a0"
        );
        assert_eq!(m.dealCreationBlock, Uint::from(86));
        // TODO: fix cids
        // let cid_bytes = [m.appCID.prefixes.to_vec(), m.appCID.hash.to_vec()].concat();
        // let app_cid = libipld::Cid::read_bytes(cid_bytes.as_slice())
        //     .unwrap()
        //     .to_string();
        // assert_eq!(
        //     app_cid,
        //     "bafkreifolrizgmusl4y7or5e5xmvr623a6i3ca4d5rwv457cezhschqj4m"
        // );
    }
}
