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

use crate::function::provider_record::{provider_to_record, record_to_provider};
use crate::function::router::Service;
use crate::FunctionRouter;
use libp2p::kad::PutRecordError;
use libp2p::kad::Quorum;
use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::string::FromUtf8Error;

impl FunctionRouter {
    pub fn dht_put_failed(&mut self, error: PutRecordError) {
        use PutRecordError::*;

        let service_id = match Self::service_id(error.key()) {
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

        if let Some((n, q)) = n_q {
            // n >= 50% of required quorum // TODO: is it reasonable?
            if 2 * n - q > 0 {
                #[rustfmt::skip]
                log::warn!("DHT.put almost failed, saved {} of {} replicas, but it is good enough", n, q);
                return;
            }
        }

        match self.local_services.entry(service_id.clone()) {
            Entry::Occupied(e) => {
                let service = e.remove();
                // remove failed record from DHT // TODO: sure need to do that?
                self.kademlia.remove_record(error.key());
                let err_msg = format!(
                    "Error while registering service {:?}: DHT.put failed: {:?}",
                    service, error
                );
                log::warn!("{}", err_msg);
                let address = match service {
                    Service::Delegated { forward_to } => forward_to,
                };
                self.send_error(address, err_msg);
            }
            Entry::Vacant(_) => {
                log::warn!(
                    "DHT put failed for service_id {}, but no registered services found",
                    service_id
                );
            }
        }
    }

    pub fn dht_get_finished(&mut self, result: libp2p::kad::GetRecordResult) {
        use libp2p::kad::{GetRecordError, GetRecordOk};

        let service_id = match &result {
            Ok(GetRecordOk { records }) if records.is_empty() => {
                debug_assert!(
                    !records.is_empty(),
                    "Got GetRecordOK, but no records returned"
                );
                log::error!("Got GetRecordOK, but no records returned, can't send error anywhere. That shouldn't happen");
                return;
            }
            Ok(GetRecordOk { records }) => {
                let first = records.first().expect("records aren't empty");
                Self::service_id(&first.key)
            }
            Err(err) => Self::service_id(err.key()),
        };

        let service_id = match &service_id {
            Ok(service_id) => service_id,
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
                let err_msg = format!("Error on DHT.get for service {}: {}", service_id, err_msg);
                log::warn!("{}", err_msg);
                self.provider_search_failed(service_id, err_msg.as_str());
                return;
            }
        };

        let records = records
            .into_iter()
            .flat_map(|rec| match crate::kademlia::expand_record_set(rec) {
                Ok(Ok(records)) => records,
                Err(record) => std::iter::once(record).collect::<HashSet<_>>(),
                Ok(Err(expand_failed)) => {
                    log::warn!(
                        "Can't expand record, skipping: {:?}; service_id {}",
                        expand_failed,
                        service_id
                    );
                    HashSet::new()
                }
            })
            .collect::<HashSet<_>>();

        let providers = records
            .into_iter()
            .flat_map(|rec| match record_to_provider(&rec) {
                Ok(Service::Delegated { forward_to }) => Some(forward_to),
                Err(err) => {
                    log::warn!(
                        "Can't deserialize provider from record {:?}, skipping: {:?}; service {}",
                        rec,
                        err,
                        service_id,
                    );
                    None
                }
            })
            .collect::<HashSet<_>>();

        self.providers_found(service_id, providers);
    }

    pub fn get_providers(&mut self, service_id: String) {
        self.kademlia
            .get_record(&service_id.as_bytes().to_vec().into(), Quorum::Majority)
    }

    pub fn publish_provider(&mut self, service_id: String, service: &Service) {
        // TODO: Quorum::All?
        self.kademlia
            .put_record(provider_to_record(service_id, service), Quorum::Majority);
    }

    fn service_id(key: &libp2p::kad::record::Key) -> Result<String, FromUtf8Error> {
        String::from_utf8(key.as_ref().to_vec())
    }
}
