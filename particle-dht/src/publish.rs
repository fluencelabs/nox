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

use crate::{dht::PublishError, DHTEvent, ParticleDHT};
use libp2p::{
    kad::{PutRecordError, PutRecordResult, Quorum, Record},
    swarm::NetworkBehaviourAction,
    PeerId,
};
use std::borrow::Borrow;

impl ParticleDHT {
    pub fn publish_client(&mut self, client: PeerId) {
        let bytes = [client.as_bytes(), self.config.peer_id.as_bytes()].concat();
        let signature = self.config.keypair.sign(bytes.as_slice());
        let key: &[u8] = client.borrow();
        let record = Record::new(key.to_vec(), signature);

        match self.kademlia.put_record(record, Quorum::Majority) {
            Ok(query_id) => {
                self.pending.insert(query_id, client);
            }
            Err(err) => self.publish_failed(client, PublishError::StoreError(err)),
        };
    }

    pub fn publish_failed(&mut self, client: PeerId, error: PublishError) {
        self.emit(DHTEvent::PublishFailed(client, error));
    }

    pub fn publish_succeeded(&mut self, client: PeerId) {
        self.emit(DHTEvent::Published(client))
    }

    pub(super) fn emit(&mut self, event: DHTEvent) {
        self.push_event(NetworkBehaviourAction::GenerateEvent(event))
    }

    pub(super) fn recover_result(result: PutRecordResult) -> Result<(), PublishError> {
        let err = match result {
            Ok(_) => return Ok(()),
            Err(err) => err,
        };

        #[rustfmt::skip]
            let (found, quorum, reason, key) = match &err {
            PutRecordError::QuorumFailed { success, quorum, key, .. } => (success.len(), quorum.get(), PublishError::QuorumFailed, key),
            PutRecordError::Timeout { success, quorum, key, .. } => (success.len(), quorum.get(), PublishError::TimedOut, key),
        };

        // TODO: is 50% a reasonable number?
        // TODO: move to config
        // Recover if found more than 50% of required quorum
        if found * 2 > quorum {
            #[rustfmt::skip]
            log::warn!("DHT.put almost failed, saved {} of {} replicas, but it is good enough", found, quorum);
            return Ok(());
        }

        let err_msg = format!(
            "Error while publishing provider for {:?}: DHT.put failed ({}/{} replies): {:?}",
            key, found, quorum, reason
        );
        log::warn!("{}", err_msg);

        Err(reason)
    }
}
