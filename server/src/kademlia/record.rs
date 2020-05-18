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

use crate::kademlia::memory_store::{MultiRecord, MultiRecordKind};
use libp2p::kad::record::Record;
use libp2p::PeerId;
use once_cell::sync::Lazy;
use prost::Message;
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use std::ops::Deref;
use std::time::{Duration, Instant};

#[derive(Clone, PartialEq, Message)]
#[prost(tags = "sequential")]
pub struct MultiRecordProto {
    #[prost(bytes, required)]
    key: Vec<u8>,
    #[prost(message, repeated)]
    values: Vec<ValueProto>,
    #[prost(uint32, optional)]
    ttl: Option<u32>,
}

impl From<MultiRecord> for MultiRecordProto {
    fn from(mrec: MultiRecord) -> Self {
        #[rustfmt::skip]
        let MultiRecord { key, values, expires, kind } = mrec;

        debug_assert_eq!(
            kind,
            MultiRecordKind::MultiRecord,
            "serializing simple multirec to proto"
        );

        let values: Vec<_> = values
            .into_iter()
            .map(|(v, p)| ValueProto::new(v, p.into_bytes()))
            .collect();

        let ttl = expires.map(|t| {
            let now = Instant::now();
            if t > now {
                (t - now).as_secs() as u32
            } else {
                1 // "1" means "already expired", "0" means "does not expire at all"
            }
        });

        Self {
            // TODO: unneeded copy, could be eliminated
            key: key.to_vec(),
            values,
            ttl,
        }
    }
}

impl TryInto<MultiRecord> for MultiRecordProto {
    type Error = Box<dyn Error>;

    fn try_into(self) -> Result<MultiRecord, Self::Error> {
        use std::io::{Error, ErrorKind};

        #[rustfmt::skip]
        let MultiRecordProto { key, values, ttl } = self;

        fn peer_id(data: Vec<u8>) -> Result<PeerId, Error> {
            PeerId::from_bytes(data).map_err(|_| ErrorKind::InvalidData.into())
        }

        let values = values
            .into_iter()
            .map(|v| {
                let publisher = peer_id(v.publisher)?;
                let result: (Vec<u8>, PeerId) = (v.value, publisher);
                Ok(result)
            })
            .collect::<Result<_, _>>()
            .map_err(|e: Error| Box::new(e))?; // TODO: get rid of explicit boxing

        let expires = ttl
            .filter(|&ttl| ttl > 0)
            .map(|ttl| Instant::now() + Duration::from_secs(ttl as u64));

        Ok(MultiRecord {
            key: key.into(),
            values,
            expires,
            kind: MultiRecordKind::MultiRecord,
        })
    }
}

#[derive(Clone, PartialEq, Message)]
#[prost(tags = "sequential")]
pub struct ValueProto {
    #[prost(bytes, required)]
    value: Vec<u8>,
    #[prost(bytes, required)]
    publisher: Vec<u8>,
}

impl ValueProto {
    pub fn new(value: Vec<u8>, publisher: Vec<u8>) -> Self {
        Self { value, publisher }
    }
}

// base58: 1UMULTPLExxRECRDSxxSTREDxxHERExx
pub const MULTIPLE_RECORD_PEER_ID_BYTES: [u8; 24] = [
    0, 22, 213, 69, 194, 41, 34, 127, 226, 180, 218, 134, 129, 165, 129, 4, 46, 90, 180, 248, 241,
    86, 134, 81,
];

pub static MULTIPLE_RECORD_PEER_ID: Lazy<PeerId> =
    Lazy::new(|| PeerId::from_bytes(MULTIPLE_RECORD_PEER_ID_BYTES.to_vec()).unwrap());

pub fn is_multiple_record(record: &Record) -> bool {
    record
        .publisher
        .as_ref()
        .map_or(false, |p| p == MULTIPLE_RECORD_PEER_ID.deref())
}

pub fn multirecord_from_bytes(bytes: Vec<u8>) -> Result<MultiRecord, Box<dyn Error>> {
    let proto = MultiRecordProto::decode(bytes.as_slice())?;
    let mrec: MultiRecord = proto.try_into()?;
    Ok(mrec)
}

pub fn try_to_multirecord(record: Record) -> Result<MultiRecord, Box<dyn std::error::Error>> {
    use std::io::{Error, ErrorKind};

    if is_multiple_record(&record) {
        multirecord_from_bytes(record.value)
    } else {
        let mut values = HashMap::new();
        let publisher = match record.publisher {
            Some(p) => p,
            None => {
                log::warn!(
                    "publisher undefined in simple record key {:?} value {:?}",
                    bs58::encode(record.key).into_string(),
                    bs58::encode(record.value).into_string()
                );
                // TODO: get rid of Box::new
                return Err(Box::new(Error::from(ErrorKind::NotFound)));
            }
        };
        values.insert(record.value, publisher);

        Ok(MultiRecord {
            key: record.key,
            values,
            expires: record.expires,
            kind: MultiRecordKind::MultiRecord,
        })
    }
}

pub fn reduce_multirecord(mrec: MultiRecord) -> Record {
    use MultiRecordKind::*;
    debug_assert!(!mrec.values.is_empty(), "mrec.values can't be empty here");

    match &mrec.kind {
        MultiRecord => {
            let key = mrec.key.clone();
            let expires = mrec.expires;

            let proto: MultiRecordProto = mrec.into();
            let mut value = Vec::with_capacity(proto.encoded_len());
            proto.encode(&mut value).expect("enough capacity");

            Record {
                key,
                value,
                publisher: Some(MULTIPLE_RECORD_PEER_ID.clone()),
                expires,
            }
        }
        SimpleRecord => {
            let (value, publisher) = mrec
                .values
                .into_iter()
                .next()
                .expect("simple record.values can't be empty");
            Record {
                key: mrec.key,
                value,
                publisher: Some(publisher),
                expires: mrec.expires,
            }
        }
    }
}
