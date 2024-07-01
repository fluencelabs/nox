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

use std::borrow::{Borrow, Cow};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use bytes::Bytes;
use libipld::multihash::{Code, MultihashDigest};
use libipld::pb::{PbLink, PbNode};
use libipld::IpldCodec::{DagPb, Raw};
use libipld::{cid, Cid};
use quick_protobuf::{MessageWrite, Writer};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::unixfs::mod_Data::DataType;
use crate::unixfs::Data as UnixFsMetadata;

/// CHUNK_SIZE is the size of the chunks that we use to split the data into before hashing.
/// 262144 is the default size used by the go-ipfs implementation.
/// It should be used everywhere in the Fluence stack to produce the same CIDs.
const CHUNK_SIZE: usize = 262144;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Hash(pub Cid);

impl Hash {
    pub fn new(bytes: &[u8]) -> eyre::Result<Self> {
        let chunks: Vec<&[u8]> = bytes.chunks(CHUNK_SIZE).collect();
        let mut links = Vec::new();
        let mut blocksizes = Vec::new();
        for chunk in chunks {
            let digest = Code::Sha2_256.digest(chunk);
            let cid = Cid::new_v1(Raw.into(), digest);
            links.push(PbLink {
                cid,
                // name for links should be empty, with None it produces results different from go-ipfs
                name: Some("".to_string()),
                size: Some(chunk.len() as u64),
            });
            blocksizes.push(chunk.len() as u64);
        }

        if links.len() == 1 {
            return Ok(Hash(links[0].cid));
        }

        let metadata = UnixFsMetadata {
            Type: DataType::File,
            filesize: Some(bytes.len() as u64),
            blocksizes,
            ..Default::default()
        };

        let mut metadata_bytes = vec![];
        let mut writer = Writer::new(&mut metadata_bytes);
        UnixFsMetadata::write_message(&metadata, &mut writer)?;

        let pb_node = PbNode {
            links,
            data: Some(Bytes::from(metadata_bytes)),
        };
        let digest = Code::Sha2_256.digest(pb_node.into_bytes().borrow());
        Ok(Hash(Cid::new_v1(DagPb.into(), digest)))
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    pub fn from_string(s: &str) -> Result<Self, cid::Error> {
        let cid = Cid::from_str(s)?;
        Ok(Self(cid))
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <Cow<'de, str>>::deserialize(deserializer)?;
        Hash::from_string(s.borrow()).map_err(de::Error::custom)
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, s: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        self.0.to_string().serialize(s)
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.to_string().fmt(f)
    }
}
