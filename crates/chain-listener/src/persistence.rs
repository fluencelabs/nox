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
use eyre::Context;
use std::fs;
use std::path::Path;

use ccp_shared::proof::ProofIdx;
use ethabi::ethereum_types::U256;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PersistedProofId {
    pub proof_id: ProofIdx,
    pub epoch: U256,
}

pub(crate) fn proof_id_filename() -> String {
    "proof_id.toml".to_string()
}

pub(crate) fn persist_proof_id(proof_id_dir: &Path, proof_id: ProofIdx, current_epoch: U256) -> eyre::Result<()> {
    let path = proof_id_dir.join(proof_id_filename());
    let bytes =
        toml::ser::to_vec(&PersistedProofId { proof_id, epoch: current_epoch }).map_err(|err| eyre::eyre!("Proof id serialization failed {err}"))?;
    Ok(fs::write(&path, bytes)
        .context(format!("error writing proof id to {}", path.display()))?)
}

pub(crate)  fn load_persisted_proof_id(proof_id_dir: &Path) -> eyre::Result<Option<PersistedProofId>> {
    let path = proof_id_dir.join(proof_id_filename());
    if path.exists() {
        let bytes = fs::read(&path)
            .context(format!("error reading proof id from {}", path.display()))?;
        let persisted_proof = toml::from_slice(&bytes)
            .context(format!("error deserializing proof id from {}", path.display()))?;
        Ok(Some(persisted_proof))
    } else {
        Ok(None)
    }
}
