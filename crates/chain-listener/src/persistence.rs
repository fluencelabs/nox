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
