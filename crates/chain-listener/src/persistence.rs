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
use std::fs;
use std::path::Path;
use std::str::FromStr;

use ccp_shared::proof::ProofIdx;

pub(crate) fn proof_id_filename() -> String {
    "proof_id.txt".to_string()
}

pub(crate) fn persist_proof_id(proof_id_dir: &Path, proof_id: ProofIdx) -> eyre::Result<()> {
    let path = proof_id_dir.join(proof_id_filename());
    Ok(fs::write(path, proof_id.to_string())?)
}

/// Load info about persisted workers from disk in parallel
pub(crate) fn load_persisted_proof_id(proof_id_dir: &Path) -> eyre::Result<ProofIdx> {
    let path = proof_id_dir.join(proof_id_filename());
    if path.exists() {
        Ok(ProofIdx::from_str(&fs::read_to_string(path)?)?)
    } else {
        log::warn!("No proof id found in {:?}, set id to 0", proof_id_dir);
        Ok(ProofIdx::zero())
    }
}
