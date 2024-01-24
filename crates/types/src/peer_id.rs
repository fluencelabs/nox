pub(crate) mod peer_id_serde {
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
