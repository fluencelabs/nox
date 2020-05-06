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

#![allow(clippy::mutable_key_type)]

use crate::function::address_signature::{remove_signatures, verify_address, SignatureError};
use crate::kademlia;
use crate::FunctionRouter;
use faas_api::{Address, AddressError};
use libp2p::{
    kad::record::{Key, Record},
    kad::{PutRecordError, Quorum},
};
use std::{
    collections::{hash_map::Entry, HashSet},
    convert::TryInto,
};

#[derive(Debug)]
pub enum ProviderError {
    Deserialization(AddressError),
    Signature(SignatureError),
}

impl FunctionRouter {
    pub fn name_resolution_failed(&mut self, error: PutRecordError) {
        use PutRecordError::*;

        let key: &Key = error.key();
        let name: Address = match key.try_into() {
            Ok(service_id) => service_id,
            Err(err) => {
                #[rustfmt::skip]
                log::warn!("Couldn't parse service_id from PutRecordError.key(): {:?}",err);
                return;
            }
        };

        #[rustfmt::skip]
        let n_q = match &error {
            QuorumFailed { num_results, quorum, .. } => Some((num_results, quorum.get())),
            Timeout { num_results, quorum, .. } => Some((num_results, quorum.get())),
            _ => None,
        };

        if let Some((&found, quorum)) = n_q {
            // found more than 50% of required quorum // TODO: is it reasonable?
            if found * 2 > quorum {
                #[rustfmt::skip]
                log::warn!("DHT.put almost failed, saved {} of {} replicas, but it is good enough", found, quorum);
                return;
            }
        }

        // Remove failed record, and send an error
        match self.provided_names.entry(name.clone()) {
            Entry::Occupied(e) => {
                let provider = e.remove();
                // remove failed record from DHT // TODO: sure need to do that?
                self.kademlia.remove_record(error.key());
                let err_msg = format!(
                    "Error while registering service {:?}: DHT.put failed: {:?}",
                    provider, error
                );
                log::warn!("{}", err_msg);
                self.send_error(provider, err_msg);
            }
            Entry::Vacant(_) => {
                log::warn!(
                    "DHT put failed for service_id {}, but no registered services found",
                    name
                );
            }
        }
    }

    pub fn name_resolved(&mut self, result: libp2p::kad::GetRecordResult) {
        use libp2p::kad::{GetRecordError, GetRecordOk};

        let name = match &result {
            Ok(GetRecordOk { records }) if records.is_empty() => {
                #[rustfmt::skip]
                debug_assert!(!records.is_empty(), "Got GetRecordOK, but no records returned");
                log::error!("Got GetRecordOK, but no records returned, can't send error anywhere. That shouldn't happen");
                return;
            }
            Ok(GetRecordOk { records }) => {
                let first = records.first().expect("records can't be empty");
                (&first.key).try_into()
            }
            Err(err) => err.key().try_into(),
        };

        let name = match &name {
            Ok(name) => name,
            Err(utf_error) => {
                log::warn!("Can't parse service_id from dht record: {:?}", utf_error);
                // TODO: maybe it's a different type of record – handle that
                return;
            }
        };

        let records = match result {
            Ok(GetRecordOk { records }) => Ok(records),
            Err(err) => match err {
                GetRecordError::NotFound { .. } => Err("Record not found".to_string()),
                GetRecordError::QuorumFailed {
                    records, quorum, ..
                } if records.is_empty() => Err(format!(
                    "Quorum failed (quorum={}), no records returned",
                    quorum
                )),
                GetRecordError::QuorumFailed { records, .. } => Ok(records),
                GetRecordError::Timeout {
                    records, quorum, ..
                } if records.is_empty() => Err(format!(
                    "Timed out (quorum={}), no records returned",
                    quorum
                )),
                GetRecordError::Timeout { records, .. } => Ok(records),
            },
        };

        let records = match records {
            Ok(records) => records,
            Err(err_msg) => {
                let err_msg = format!("Error on DHT.get for service {}: {}", name, err_msg);
                log::warn!("{}", err_msg);
                self.provider_search_failed(name, err_msg.as_str());
                return;
            }
        };

        let records = records
            .into_iter()
            .flat_map(|rec| match kademlia::expand_record_set(rec) {
                Ok(Ok(records)) => records,
                Err(record) => std::iter::once(record).collect::<HashSet<_>>(),
                Ok(Err(expand_failed)) => {
                    log::warn!(
                        "Can't expand record, skipping: {:?}; service_id {}",
                        expand_failed,
                        name
                    );
                    HashSet::new()
                }
            })
            .collect::<HashSet<_>>();

        let providers = records
            .into_iter()
            .flat_map(|rec| match Self::deserialize_provider(&rec) {
                Ok(provider) => Some(provider),
                Err(err) => {
                    log::warn!(
                        "Can't deserialize provider from record {:?}, skipping: {:?}; service {}",
                        rec,
                        err,
                        name,
                    );
                    None
                }
            })
            .collect::<HashSet<_>>();

        self.providers_found(name, providers);
    }

    fn deserialize_provider(record: &Record) -> Result<Address, ProviderError> {
        use ProviderError::*;

        let slice = record.value.as_slice();
        let address: Address = slice.try_into().map_err(|e| Deserialization(e))?;
        verify_address(&address).map_err(|e| Signature(e))?;
        let address = remove_signatures(address);

        Ok(address)
    }

    // Like DNS CNAME record
    pub(super) fn publish_name(&mut self, name: Address, provider: Address) {
        self.kademlia
            .put_record(Record::new(name, provider.into()), Quorum::Majority)
    }

    pub(super) fn resolve_name(&mut self, name: Address) {
        self.kademlia.get_record(&name.into(), Quorum::Majority)
    }
}
