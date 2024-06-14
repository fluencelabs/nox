/*
 * Copyright 2024 Fluence DAO
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

use crate::persistence;
use crate::persistence::PersistedProofTracker;
use alloy_primitives::{Uint, U256};
use backoff::future::retry;
use backoff::ExponentialBackoff;
use ccp_shared::proof::ProofIdx;
use ccp_shared::types::{GlobalNonce, CUID};
use eyre::eyre;
use std::cmp::max;
use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::path::PathBuf;
use std::time::Duration;

pub struct ProofTracker {
    persisted_proof_id_dir: PathBuf,
    current_epoch: U256,
    global_nonce: GlobalNonce,
    proof_counter: BTreeMap<CUID, U256>,
    last_submitted_proof_ids: HashMap<CUID, ProofIdx>,
}

impl ProofTracker {
    pub fn new(persisted_proof_id_dir: PathBuf) -> Self {
        Self {
            persisted_proof_id_dir,
            current_epoch: U256::ZERO,
            global_nonce: GlobalNonce::new([0; 32]),
            proof_counter: BTreeMap::new(),
            last_submitted_proof_ids: HashMap::new(),
        }
    }

    pub async fn load_state(&mut self) -> eyre::Result<()> {
        let persisted_proof_tracker =
            persistence::load_persisted_proof_tracker(&self.persisted_proof_id_dir).await?;

        if let Some(state) = persisted_proof_tracker {
            tracing::info!(target: "chain-listener", "Loaded proof tracker state");
            self.proof_counter = state.proof_counter.into_iter().collect();
            self.last_submitted_proof_ids = state
                .proof_ids
                .into_iter()
                .collect::<HashMap<CUID, ProofIdx>>();
            self.global_nonce = state.global_nonce;
            self.current_epoch = state.epoch;
        } else {
            tracing::info!(target: "chain-listener", "No persisted proof tracker state found")
        }

        Ok(())
    }

    pub async fn observe_proof(&mut self, cu_id: CUID, proof_id: ProofIdx) {
        self.last_submitted_proof_ids
            .entry(cu_id)
            .and_modify(|id| *id = max(*id, proof_id))
            .or_insert(proof_id);

        tracing::info!(target: "chain-listener", "Persisted proof id {} for {} on epoch {} nonce {}", proof_id, cu_id, self.current_epoch, self.global_nonce);

        self.persist().await;
    }

    pub fn get_last_submitted_proof_id(&self, cu_id: &CUID) -> ProofIdx {
        self.last_submitted_proof_ids
            .get(cu_id)
            .copied()
            .unwrap_or(ProofIdx::zero())
    }

    pub async fn confirm_proof(&mut self, cu_id: CUID) -> U256 {
        let proof_id = Uint::from(
            *self
                .proof_counter
                .entry(cu_id)
                .and_modify(|c| {
                    *c = c.add(Uint::from(1));
                })
                .or_insert(Uint::from(1)),
        );
        self.persist().await;

        proof_id
    }

    pub fn get_proof_counter(&self, cu_id: &CUID) -> U256 {
        self.proof_counter.get(cu_id).copied().unwrap_or(Uint::ZERO)
    }

    pub fn get_proof_counters(&self) -> BTreeMap<CUID, U256> {
        self.proof_counter.clone()
    }
    pub async fn set_current_epoch(&mut self, epoch_number: U256) {
        if self.current_epoch != epoch_number {
            tracing::info!(target: "chain-listener", "Epoch changed, was {}, new epoch number is {epoch_number}", self.current_epoch);
            self.current_epoch = epoch_number;
            self.proof_counter.clear();
            self.persist().await;
        }
    }

    pub async fn set_global_nonce(&mut self, global_nonce: GlobalNonce) {
        if self.global_nonce != global_nonce {
            tracing::info!(target: "chain-listener", "Global changed, was {}, new global nonce is {global_nonce}", self.global_nonce);
            self.global_nonce = global_nonce;
            tracing::info!(target: "chain-listener", "Resetting proof id counter");
            self.last_submitted_proof_ids.clear();
            self.persist().await;
        }
    }

    async fn persist(&self) {
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(3)),
            ..ExponentialBackoff::default()
        };

        let write = retry(backoff, || async {
            persistence::persist_proof_tracker(
                &self.persisted_proof_id_dir,
                &self
            ).await.map_err(|err|{
                tracing::warn!(target: "chain-listener", "Failed to persist proof tracker state: {err}; Retrying...");
                eyre!(err)
            })?;
            Ok(())
        }).await;

        if let Err(err) = write {
            tracing::warn!(target: "chain-listener", "Failed to persist proof tracker state: {err}; Ignoring..");
        } else {
            tracing::debug!(target: "chain-listener", "Proof tracker state persisted successfully");
        }
    }
}

impl From<&ProofTracker> for PersistedProofTracker {
    fn from(tracker: &ProofTracker) -> Self {
        Self {
            proof_ids: tracker
                .last_submitted_proof_ids
                .clone()
                .into_iter()
                .collect(),
            proof_counter: tracker.proof_counter.clone().into_iter().collect(),
            global_nonce: tracker.global_nonce.clone(),
            epoch: tracker.current_epoch.clone(),
        }
    }
}
