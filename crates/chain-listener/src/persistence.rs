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

use alloy_primitives::U256;
use eyre::Context;
use hex_utils::serde_as::Hex;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::path::Path;

use crate::proof_tracker::ProofTracker;
use alloy_serde_macro::{U256_as_String, U256_from_String};
use ccp_shared::proof::ProofIdx;
use ccp_shared::types::{GlobalNonce, CUID};
use serde::{Deserialize, Serialize};

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct PersistedProofTracker {
    #[serde_as(as = "Vec<(Hex,DisplayFromStr)>")]
    pub proof_ids: Vec<(CUID, ProofIdx)>,
    #[serde_as(as = "Vec<(Hex,DisplayFromStr)>")]
    pub proof_counter: Vec<(CUID, U256)>,
    #[serde_as(as = "Hex")]
    pub global_nonce: GlobalNonce,
    #[serde(
        serialize_with = "U256_as_String",
        deserialize_with = "U256_from_String"
    )]
    pub epoch: U256,
}

pub(crate) fn proof_tracker_state_filename() -> String {
    "proof_tracker.toml".to_string()
}

pub(crate) async fn persist_proof_tracker(
    proof_id_dir: &Path,
    proof_tracker: &ProofTracker,
) -> eyre::Result<()> {
    let path = proof_id_dir.join(proof_tracker_state_filename());
    let bytes = toml_edit::ser::to_vec(&PersistedProofTracker::from(proof_tracker))
        .map_err(|err| eyre::eyre!("Proof tracker serialization failed {err}"))?;
    tokio::fs::write(&path, bytes)
        .await
        .context(format!("error writing proof id to {}", path.display()))
}

pub(crate) async fn load_persisted_proof_tracker(
    proof_tracker_dir: &Path,
) -> eyre::Result<Option<PersistedProofTracker>> {
    let path = proof_tracker_dir.join(proof_tracker_state_filename());
    if path.exists() {
        let bytes = tokio::fs::read(&path).await.context(format!(
            "error reading proof tracker state from {}",
            path.display()
        ))?;
        let persisted_proof = toml_edit::de::from_slice(&bytes).context(format!(
            "error deserializing proof tracker state from {}, content {}",
            path.display(),
            String::from_utf8_lossy(&bytes)
        ))?;
        Ok(Some(persisted_proof))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tempfile::tempdir;

    #[tokio::test]
    async fn proof_tracker_test() {
        let dir = tempdir().expect("Could not create temp dir");
        let mut proof_tracker = ProofTracker::new(dir.path().to_path_buf());
        let cuid =
            CUID::from_str("7dcee6bb1c39396de3b19424154ad3996cbdef5f3950022325bb4f651e48fbe0")
                .unwrap();

        let global_nonce = GlobalNonce::new([1; 32]);
        let epoch = U256::from(100);
        let proof_id = ProofIdx::from_str("1").unwrap();
        proof_tracker.set_current_epoch(epoch).await;
        proof_tracker.set_global_nonce(global_nonce).await;

        proof_tracker.observe_proof(cuid, proof_id).await;

        proof_tracker.confirm_proof(cuid).await;
        proof_tracker.confirm_proof(cuid).await;

        let persisted_proof_tracker = load_persisted_proof_tracker(dir.path())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(persisted_proof_tracker.proof_ids.len(), 1);
        assert_eq!(persisted_proof_tracker.proof_ids[0].0, cuid);
        assert_eq!(persisted_proof_tracker.proof_ids[0].1, proof_id);

        assert_eq!(persisted_proof_tracker.proof_counter.len(), 1);
        assert_eq!(persisted_proof_tracker.proof_counter[0].0, cuid);
        assert_eq!(persisted_proof_tracker.proof_counter[0].1, U256::from(2));

        assert_eq!(persisted_proof_tracker.epoch, epoch);
        assert_eq!(persisted_proof_tracker.global_nonce, global_nonce);
    }
}
