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

use crate::errors::{ResolveError, ResolveErrorKind};
use crate::wait_peer::WaitPeer;
use crate::{DHTEvent, ParticleDHT};

use particle_protocol::Particle;

use libp2p::{
    kad::{record::Key, GetRecordError, GetRecordOk, GetRecordResult, PeerRecord, Quorum},
    PeerId,
};

impl ParticleDHT {
    pub fn resolve(&mut self, key: Key) {
        self.kademlia.get_record(&key, Quorum::Majority);
    }

    pub(super) fn resolved(&mut self, result: GetRecordResult) {
        match Self::recover_get(result) {
            Ok(records) => {
                // unwrapping is safe here because recover_get returns a non-empty vector of PeerRecords
                let key = records.iter().next().unwrap().record.key.clone();
                let value = records.into_iter().map(|r| r.record.value).collect();
                self.emit(DHTEvent::Resolved { key, value });
            }
            Err(err) => self.emit(DHTEvent::ResolveFailed { err }),
        }
    }

    /// Returns either a non-empty vector of `PeerRecord`, or an error
    pub(super) fn recover_get(result: GetRecordResult) -> Result<Vec<PeerRecord>, ResolveError> {
        let err = match result {
            Err(err) => err,
            Ok(GetRecordOk { records }) if records.is_empty() => {
                unreachable!("It should be GetRecordError::NotFound")
            }
            Ok(GetRecordOk { records }) => return Ok(records),
        };

        #[rustfmt::skip]
        let (found, quorum, reason, key) = match err {
            GetRecordError::QuorumFailed { records, quorum, key, .. } => (records, quorum.get(), ResolveErrorKind::QuorumFailed, key),
            GetRecordError::Timeout { records, quorum, key, .. } => (records, quorum.get(), ResolveErrorKind::TimedOut, key),
            GetRecordError::NotFound { key, .. } => (vec![], 0, ResolveErrorKind::NotFound, key)
        };

        // TODO: is 50% a reasonable number?
        // TODO: move to config
        // Recover if found more than 50% of required quorum
        if found.len() * 2 > quorum {
            #[rustfmt::skip]
            log::warn!("DHT.get almost failed, got {} of {} replicas, but it is good enough", found.len(), quorum);
            return Ok(found);
        }

        #[rustfmt::skip]
        let err_msg = format!("Error while resolving key {:?}: DHT.get failed ({}/{} replies): {:?}", key, found.len(), quorum, reason);
        log::warn!("{}", err_msg);

        Err(ResolveError { key, kind: reason })
    }

    /// Look for peer in Kademlia, enqueue call to wait for result
    pub(super) fn search_then_send(&mut self, peer_id: PeerId, particle: Particle) {
        self.query_closest(peer_id, WaitPeer::Routable(particle))
    }

    /// Query for peers closest to the `peer_id` as DHT key, enqueue call until response
    fn query_closest(&mut self, peer_id: PeerId, call: WaitPeer) {
        log::debug!("Queried closest peers for peer_id {}", peer_id);
        // TODO: Don't call get_closest_peers if there are already WaitPeer::Neighbourhood or WaitPeer::Routable enqueued
        self.wait_peer.enqueue(peer_id.clone(), call);
        // NOTE: Using Qm form of `peer_id` here (via peer_id.borrow), since kademlia uses that for keys
        self.kademlia.get_closest_peers(peer_id);
    }

    pub(super) fn found_closest(&mut self, peer_id: PeerId, peers: Vec<PeerId>) {
        log::debug!("Found closest peers for peer_id {}: {:?}", peer_id, peers);
        // Forward to `peer_id`
        let particles = self
            .wait_peer
            .remove_with(peer_id.clone(), |wp| wp.routable());

        if peers.is_empty() || !peers.iter().any(|p| p == &peer_id) {
            for _particle in particles {
                let err_msg = format!(
                    "peer {} wasn't found via closest query: {:?}",
                    peer_id,
                    peers.iter().map(|p| p.to_base58())
                );
                log::warn!("{}", err_msg);
                // Peer wasn't found, send error
                // self.send_error_on_call(particle.into(), err_msg)
                unimplemented!("error handling")
            }
        } else {
            for particle in particles {
                // Forward calls to `peer_id`, guaranteeing it is now routable
                self.connect_then_send(peer_id.clone(), particle.into())
            }
        }
    }
}
