/*
 * Copyright 2020 Fluence Labs Limited
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

// This module is intended to provide a way to serialize/deserialize PeerId and Multihash.

pub mod peerid_serializer {
    use libp2p::PeerId;
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
            serde::de::Error::custom(format!("peer id deserialization failed for {:?}", e))
        })
    }
}

pub mod multihash_serializer {
    use multihash::Multihash;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Multihash, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.as_bytes().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Multihash, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<u8>::deserialize(deserializer)?;
        Multihash::from_bytes(vec).map_err(|e| {
            serde::de::Error::custom(format!("peer id deserialization failed with {:?}", e))
        })
    }
}

// waiting for https://github.com/serde-rs/serde/issues/723 that will make UX better
pub mod provider_serializer {
    use libp2p::PeerId;
    use parity_multiaddr::Multiaddr;
    use serde::de::{SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serializer};

    use std::fmt;
    use std::str::FromStr;

    #[allow(clippy::ptr_arg)]
    pub fn serialize<S>(value: &Vec<(Multiaddr, PeerId)>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2 * value.len()))?;
        for (multiaddr, peerid) in value {
            seq.serialize_element(multiaddr)?;
            seq.serialize_element(&peerid.to_base58())?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<(Multiaddr, PeerId)>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VecVisitor;
        impl<'de> Visitor<'de> for VecVisitor {
            type Value = Vec<(Multiaddr, PeerId)>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("[Multiaddr, PeerId]")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut vec = Vec::new();

                let mut raw_multiaddr = seq.next_element::<Multiaddr>()?;
                while raw_multiaddr.is_some() {
                    let multiaddr = raw_multiaddr.unwrap();
                    let peer_id_bytes = seq.next_element::<String>()?;

                    if let Some(peer_id) = peer_id_bytes {
                        let peer_id = PeerId::from_str(&peer_id).map_err(|e| {
                            serde::de::Error::custom(format!(
                                "peer id deserialization failed for {:?}",
                                e
                            ))
                        })?;
                        vec.push((multiaddr, peer_id));
                    } else {
                        // Multiaddr deserialization's been successfull, but PeerId hasn't - return a error
                        return Err(serde::de::Error::custom("failed to deserialize PeerId"));
                    }

                    raw_multiaddr = seq.next_element::<Multiaddr>()?;
                }

                Ok(vec)
            }
        }

        deserializer.deserialize_seq(VecVisitor {})
    }
}

#[cfg(test)]
mod tests {
    use libp2p::PeerId;
    use serde::{Deserialize, Serialize};
    use std::str::FromStr;

    #[test]
    fn peerid() {
        use crate::misc::peerid_serializer;

        #[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
        struct Test {
            #[serde(with = "peerid_serializer")]
            peer_id_1: PeerId,
            #[serde(with = "peerid_serializer")]
            peer_id_2: PeerId,
        };

        let peer_id_1 = PeerId::random();
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

    #[test]
    fn multihash() {
        use crate::misc::multihash_serializer;
        use multihash::Multihash;

        #[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
        struct Test {
            #[serde(with = "multihash_serializer")]
            multihash_1: Multihash,
            #[serde(with = "multihash_serializer")]
            multihash_2: Multihash,
        };

        let peer_id_1 = PeerId::random();
        let peer_id_2 = PeerId::from_str("QmY28NSCefB532XbERtnKHadexGuNzAfYnh5fJk6qhLsSi").unwrap();

        let test = Test {
            multihash_1: Multihash::from(peer_id_1),
            multihash_2: Multihash::from(peer_id_2),
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

    #[test]
    fn providers() {
        use crate::misc::provider_serializer;
        use parity_multiaddr::Multiaddr;
        use std::net::Ipv4Addr;

        #[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
        struct Test {
            #[serde(with = "provider_serializer")]
            providers: Vec<(Multiaddr, PeerId)>,
        };

        let mut providers = Vec::new();
        let mut test_peer_ids = Vec::new();
        providers.push((
            Multiaddr::from(Ipv4Addr::new(0, 0, 0, 0)),
            PeerId::from_str("QmY28NSCefB532XbERtnKHadexGuNzAfYnh5fJk6qhLsSi").unwrap(),
        ));

        for i in 1..=255 {
            let peer_id = PeerId::random();

            providers.push((Multiaddr::from(Ipv4Addr::new(i, i, i, i)), peer_id.clone()));
            test_peer_ids.push(peer_id);
        }

        let test = Test { providers };

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
