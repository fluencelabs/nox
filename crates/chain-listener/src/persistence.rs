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
use std::path::Path;

use alloy_serde_macro::{U256_as_String, U256_from_String};
use ccp_shared::proof::ProofIdx;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct PersistedProofId {
    pub proof_id: ProofIdx,
    #[serde(
        serialize_with = "U256_as_String",
        deserialize_with = "U256_from_String"
    )]
    pub epoch: U256,
}

pub(crate) fn proof_id_filename() -> String {
    "proof_id.toml".to_string()
}

pub(crate) async fn persist_proof_id(
    proof_id_dir: &Path,
    proof_id: ProofIdx,
    current_epoch: U256,
) -> eyre::Result<()> {
    let path = proof_id_dir.join(proof_id_filename());
    let bytes = toml_edit::ser::to_vec(&PersistedProofId {
        proof_id,
        epoch: current_epoch,
    })
    .map_err(|err| eyre::eyre!("Proof id serialization failed {err}"))?;
    tokio::fs::write(&path, bytes)
        .await
        .context(format!("error writing proof id to {}", path.display()))
}

pub(crate) async fn load_persisted_proof_id(
    proof_id_dir: &Path,
) -> eyre::Result<Option<PersistedProofId>> {
    let path = proof_id_dir.join(proof_id_filename());
    if path.exists() {
        let bytes = tokio::fs::read(&path)
            .await
            .context(format!("error reading proof id from {}", path.display()))?;
        let persisted_proof = toml_edit::de::from_slice(&bytes).context(format!(
            "error deserializing proof id from {}",
            path.display()
        ))?;
        Ok(Some(persisted_proof))
    } else {
        Ok(None)
    }
}
