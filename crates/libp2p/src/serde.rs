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

// This module is intended to provide a way to serialize/deserialize PeerId and Multihash.

pub mod peerid_serializer_opt {
    use libp2p::PeerId;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::str::FromStr;

    pub fn serialize<S>(value: &Option<PeerId>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.map(|p| p.to_base58()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<PeerId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt {
            None => Ok(None),
            Some(str) => PeerId::from_str(&str)
                .map_err(|e| {
                    serde::de::Error::custom(format!("peer id deserialization failed for {e:?}"))
                })
                .map(Some),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::RandomPeerId;
    use libp2p::PeerId;
    use serde::{Deserialize, Serialize};
    use std::str::FromStr;

    #[test]
    fn multihash() {
        use multihash::Multihash;

        #[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
        struct Test {
            multihash_1: Multihash<64>,
            multihash_2: Multihash<64>,
        }

        let peer_id_1 = RandomPeerId::random();
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
}
