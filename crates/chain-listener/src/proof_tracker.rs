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

use crate::persistence;
use crate::persistence::PersistedProofTracker;
use alloy_primitives::{Uint, U256};
use ccp_shared::proof::ProofIdx;
use ccp_shared::types::{GlobalNonce, CUID};
use std::cmp::max;
use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::path::PathBuf;

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
        let cu_proof_counter = self.proof_counter.entry(cu_id).or_insert(Uint::ZERO);
        *cu_proof_counter = cu_proof_counter.add(Uint::from(1));

        let counter = *cu_proof_counter;
        self.persist().await;

        counter
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

    /// Returns true if the global nonce has changed
    pub async fn set_global_nonce(&mut self, global_nonce: GlobalNonce) -> bool {
        if self.global_nonce != global_nonce {
            tracing::info!(target: "chain-listener", "Global nonce changed, was {}, new global nonce is {global_nonce}", self.global_nonce);
            self.global_nonce = global_nonce;
            tracing::info!(target: "chain-listener", "Resetting proof id counter");
            self.last_submitted_proof_ids.clear();
            self.persist().await;
            true
        } else {
            false
        }
    }

    async fn persist(&self) {
        let write = persistence::persist_proof_tracker(&self.persisted_proof_id_dir, &self).await;

        if let Err(err) = write {
            tracing::warn!(target: "chain-listener", "Failed to persist proof tracker state: {err}");
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
