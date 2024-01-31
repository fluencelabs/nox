pub mod serde {
    use libp2p_identity::PeerId;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::str::FromStr;

    pub fn serialize<S>(value: &PeerId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.to_base58().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<PeerId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        PeerId::from_str(&str).map_err(|e| {
            serde::de::Error::custom(format!("peer id deserialization failed for {e:?}"))
        })
    }
}

#[cfg(test)]
mod tests{
    use std::str::FromStr;
    use libp2p_identity::{Keypair, PeerId};
    use serde::{Deserialize, Serialize};
    use crate::peer_id;

    #[test]
    fn peerid() {

        #[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
        struct Test {
            #[serde(serialize_with = "peer_id::serde::serialize", deserialize_with = "peer_id::serde::deserialize")]
            peer_id_1: PeerId,
            #[serde(serialize_with = "peer_id::serde::serialize", deserialize_with = "peer_id::serde::deserialize")]
            peer_id_2: PeerId,
        }

        let peer_id_1 = Keypair::generate_ed25519().public().to_peer_id();
        let peer_id_2 = PeerId::from_str("QmY28NSCefB532XbERtnKHadexGuNzAfYnh5fJk6qhLsSi").unwrap();

        let test = Test {
            peer_id_1,
            peer_id_2,
        };

        let serialized_test = serde_json::to_value(test.clone());
        assert!(
            serialized_test.is_ok(),
            "failed to serialize test struct: {}",
            serialized_test.err().unwrap()
        );

        let deserialized_test = serde_json::from_value::<Test>(serialized_test.unwrap());
        assert!(
            deserialized_test.is_ok(),
            "failed to deserialize test struct: {}",
            deserialized_test.err().unwrap()
        );
        assert_eq!(deserialized_test.unwrap(), test);
    }

}
