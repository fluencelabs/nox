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

use crate::ParticleDHT;

use control_macro::get_return;

use libp2p::{
    kad::{GetClosestPeersError, GetClosestPeersOk, KademliaEvent, QueryResult},
    swarm::NetworkBehaviourEventProcess,
    PeerId,
};

impl NetworkBehaviourEventProcess<KademliaEvent> for ParticleDHT {
    fn inject_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::QueryResult {
                id,
                result: QueryResult::PutRecord(result),
                ..
            } => {
                let client = get_return!(self.pending.remove(&id));
                if let Err(err) = Self::recover_result(result) {
                    self.publish_failed(client, err)
                } else {
                    self.publish_succeeded(client)
                }
            }
            KademliaEvent::QueryResult {
                result: QueryResult::GetClosestPeers(result),
                ..
            } => {
                let (key, peers) = match result {
                    Ok(GetClosestPeersOk { key, peers }) => (key, peers),
                    Err(GetClosestPeersError::Timeout { key, peers }) => (key, peers),
                };
                let peer_id = match PeerId::from_bytes(key) {
                    Err(err) => {
                        log::warn!(
                            "Found closest peers for invalid key {}: not a PeerId",
                            bs58::encode(err).into_string()
                        );
                        return;
                    }
                    Ok(peer_id) => peer_id,
                };

                if self.is_local(&peer_id) {
                    self.bootstrap_finished();
                } else {
                    self.found_closest(peer_id, peers);
                }
            }
            KademliaEvent::QueryResult {
                result: QueryResult::GetRecord(result),
                ..
            } => self.resolved(result),
            _ => {}
        }
    }
}
