/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use libp2p::kad::protocol::{record_from_proto, record_to_proto};
use libp2p::kad::record::{Key, Record};
use libp2p::kad::ProtoRecords;
use libp2p::PeerId;
use once_cell::sync::Lazy;
use prost::Message;
use std::collections::HashSet;
use std::error::Error;
use std::ops::Deref;

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

pub fn records_to_bytes(records: Vec<Record>) -> Vec<u8> {
    let proto = ProtoRecords {
        records: records.into_iter().map(record_to_proto).collect(),
    };
    let mut buf = Vec::with_capacity(proto.encoded_len());
    proto
        .encode(&mut buf)
        .expect("Vec<u8> provides capacity as needed");
    buf
}

pub fn records_from_bytes(bytes: Vec<u8>) -> Result<Vec<Record>, Box<dyn Error>> {
    let proto = ProtoRecords::decode(bytes.as_slice())?;
    let records = proto
        .records
        .into_iter()
        .map(record_from_proto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

pub fn expand_record_set(
    record: Record,
) -> Result<Result<HashSet<Record>, Box<dyn std::error::Error>>, Record> {
    if is_multiple_record(&record) {
        Ok(records_from_bytes(record.value).map(|rs| rs.into_iter().collect::<HashSet<_>>()))
    } else {
        Err(record)
    }
}

pub fn reduce_record_set(key: Key, set: HashSet<Record>) -> Record {
    debug_assert!(!set.is_empty(), "record set can't be empty here");

    let mut record = Record {
        key,
        value: vec![],
        publisher: Some(MULTIPLE_RECORD_PEER_ID.clone()),
        expires: None, // TODO: ???
    };

    let records = set.into_iter().collect::<Vec<_>>();
    record.value = records_to_bytes(records);

    record
}
