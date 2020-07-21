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

#![allow(clippy::mutable_key_type)]

use crate::function::provider_record::ProviderRecord;
use crate::function::wait_address::WaitAddress;
use crate::kademlia;
use crate::FunctionRouter;
use faas_api::{Address, AddressError, FunctionCall};
use libp2p::{
    kad::record::{Key, Record},
    kad::{PutRecordError, Quorum},
};
use serde_json::json;
use std::num::NonZeroUsize;
use std::{collections::HashSet, convert::TryInto};

// TODO: move GET_QUORUM_N to config
/// Number of nodes required to respond for DHT get operations
const GET_QUORUM_N: usize = 3;

impl FunctionRouter {
    /// Called when Kademlia failed to execute PutRecord.
    /// If at least 50% of required quorum was stored in DHT,
    /// then consider this a non-failure, and proceed as with success.
    /// Otherwise, consider this a failure, remove failed record from (local) DHT,
    /// and send error to provider of this name.
    ///
    /// Assumes that key is a valid `Address`, logs an error otherwise.
    pub(super) fn name_publish_failed(&mut self, error: PutRecordError) {
        use PutRecordError::*;

        let key: &Key = error.key();
        let name = Self::key_to_addr(key);
        let name: Address = match name {
            Ok(service_id) => service_id,
            _ => return,
        };

        #[rustfmt::skip]
        let (found, quorum) = match &error {
            QuorumFailed { success, quorum, .. } => (success.len(), quorum.get()),
            Timeout { success, quorum, .. } => (success.len(), quorum.get()),
        };

        // TODO: is 50% a reasonable number?
        // TODO: move to config
        // found more than 50% of required quorum
        if found * 2 > quorum {
            #[rustfmt::skip]
            log::warn!("DHT.put almost failed, saved {} of {} replicas, but it is good enough", found, quorum);
            return;
        }

        let err_msg = format!(
            "Error while publishing provider for {:?}: DHT.put failed: {:?}",
            name, error
        );
        log::warn!("{}", err_msg);

        // Remove failed record
        self.provided_names.remove(&name);
        // Send errors
        let calls = self.wait_address.remove_with(name, WaitAddress::published);
        for c in calls {
            self.send_error_on_call(c.call(), err_msg.clone())
        }
    }

    /// Called when Kademlia finished (either with success or a failure)
    /// executing GetRecord query.
    /// If GetRecord failed but returned some results, consider it a non-failure,
    /// and proceed as with success.
    /// Assume that key is a correct `Address`, log error and exit otherwise.
    /// Attempt to deserialize `Records` from each `Record` (that is, multiple
    /// records are stored inside single record), fallback to single `Record`.
    /// Then deserialize `Address` from each `Record::value` and verify signatures.
    /// All failed records are skipped over.
    pub fn name_resolved(&mut self, result: libp2p::kad::GetRecordResult) {
        use libp2p::kad::{GetRecordError, GetRecordOk};
        use GetRecordError::*;

        // Get Address name from record, or return an error
        let name = match &result {
            Ok(GetRecordOk { records }) if records.is_empty() => {
                #[rustfmt::skip]
                debug_assert!(!records.is_empty(), "Got GetRecordOK, but no records returned");
                log::error!("Got GetRecordOK, but no records returned, can't send error anywhere. That shouldn't happen");
                return;
            }
            Ok(GetRecordOk { records }) => {
                let first = records.first().expect("records can't be empty");
                (&first.record.key).try_into()
            }
            Err(err) => err.key().try_into(),
        };

        // Check name was deserialized, otherwise log error and exit
        let name = match name {
            Ok(name) => name,
            Err(err) => {
                log::warn!("Can't parse Address name from dht record key: {:?}", err);
                // TODO: maybe it's a different type of record – handle that
                return;
            }
        };

        #[rustfmt::skip]
        // Take records from success or failure, return error if there is none
        let records = match result {
            Ok(GetRecordOk { records }) => Ok(records),
            Err(err) => match err {
                QuorumFailed { records, quorum, .. } => {
                    if records.is_empty() {
                        Err(format!("Quorum failed (quorum={}), got 0 records", quorum))
                    } else {
                        Ok(records)
                    }
                }
                Timeout { records, quorum, .. } => {
                    if records.is_empty() {
                        Err(format!("Timed out (quorum={}), got 0 records", quorum))
                    } else {
                        Ok(records)
                    }
                }
                NotFound { .. } => Err("Record not found".into()),
            },
        };

        // Check there are records, and if not – fail provider search
        let records = match records {
            Ok(records) => records,
            Err(err_msg) => {
                let err_msg = format!("Error on DHT.get for name {}: {}", name, err_msg);
                log::warn!("{}", err_msg);
                self.provider_search_failed(name, err_msg.as_str());
                return;
            }
        };

        // Attempt to deserialize multiple record from each Record (expand),
        // fallback to single record, skip failed
        let records = records
            .into_iter()
            .flat_map(|rec| match kademlia::try_to_multirecord(rec.record) {
                Ok(multirec) => Some(multirec),
                Err(err) => {
                    log::warn!(
                        "Can't deserialize multirecord, skipping: {:?}; name {}",
                        err,
                        name
                    );
                    None
                }
            })
            .collect::<Vec<_>>();

        log::debug!("Found {} records for name {}", records.len(), name);

        // Deserialize provider addresses from records (also check signatures on deserialization),
        // skip failed
        let providers = records
            .iter()
            .flat_map(|rec| {
                let name = &name;
                rec.values.iter().flat_map(
                    move |(value, publisher)| {
                        match ProviderRecord::deserialize_address(value, publisher) {
                            Ok(provider) => Some(provider),
                            Err(err) => {
                                #[rustfmt::skip]
                                log::warn!(
                                    "Can't deserialize provider from record {:?}, skipping: {:?}; name {}",
                                    rec, err, name,
                                );
                                None
                            }
                        }
                    },
                )
            })
            .collect::<HashSet<_>>();

        // Proceed to send calls to found providers
        self.providers_found(name, providers);
    }

    /// Publish provider by name to dht. Similar to DNS CNAME.
    pub(super) fn publish_name(
        &mut self,
        name: Address,
        provider: &Address,
        call: Option<FunctionCall>,
    ) -> Result<(), libp2p::kad::store::Error> {
        if let Some(call) = call {
            self.wait_address
                .enqueue(name.clone(), WaitAddress::Published(call));
        }

        let record = ProviderRecord::signed(provider, &self.config.keypair);
        self.kademlia
            .put_record(Record::new(&name, record.into()), Quorum::Majority)
            .map(|_| ())
    }

    /// Send a reply signaling publishing succeeded
    pub(super) fn name_publish_succeeded(&mut self, key: Key) -> Option<()> {
        let name = Self::key_to_addr(&key).ok()?;
        let calls = self
            .wait_address
            .remove_with(name, WaitAddress::published)
            .map(|c| c.call());
        for call in calls {
            if let Err(e) = self.reply_with(call, None, ("ok", json!({}))) {
                log::warn!(
                    "Failed to send success reply after publish: {}",
                    e.err_msg()
                );
            }
        }

        Some(())
    }

    /// Find provider by name, result will eventually be delivered in `name_resolved` function
    pub(super) fn resolve_name(&mut self, name: &Address) {
        self.kademlia.get_record(
            &name.into(),
            Quorum::N(NonZeroUsize::new(GET_QUORUM_N).unwrap()),
        );
    }

    /// Remove ourselves from providers of this record, and replicate this to DHT
    pub(super) fn unpublish_name(&mut self, name: Address) {
        let key = (&name).into();
        self.kademlia.remove_record(&key);
        self.kademlia.replicate_record(key)
    }

    fn key_to_addr(key: &Key) -> Result<Address, AddressError> {
        key.try_into().map_err(|e| {
            #[rustfmt::skip]
            log::warn!("Couldn't parse provider address from DHT record key: {:?}", e);
            AddressError::from(e)
        })
    }
}
