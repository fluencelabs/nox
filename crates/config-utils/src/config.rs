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
use libp2p_identity::Keypair;
use libp2p_identity::PeerId;
use std::path::{Path, PathBuf};

pub fn workdir(base_dir: &Path) -> PathBuf {
    base_dir.join("workdir")
}

pub fn modules_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("modules")
}

pub fn services_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("services")
}

pub fn particles_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("particles")
}

pub fn particles_vault_dir(base_dir: &Path) -> PathBuf {
    particles_dir(base_dir).join("vault")
}

pub fn particles_anomaly_dir(base_dir: &Path) -> PathBuf {
    particles_dir(base_dir).join("anomalies")
}

pub fn blueprint_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("blueprint")
}

// TODO: move to fluence-identity crate
pub fn to_peer_id(kp: &Keypair) -> PeerId {
    PeerId::from(kp.public())
}
