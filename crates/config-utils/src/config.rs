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

use libp2p_core::{identity::Keypair, PeerId};
use std::path::{Path, PathBuf};

pub fn workdir(path: &Path) -> PathBuf {
    path.join("workdir")
}
pub fn modules_dir(path: &Path) -> PathBuf {
    path.join("modules")
}
pub fn services_dir(path: &Path) -> PathBuf {
    path.join("services")
}
pub fn particles_dir(path: &Path) -> PathBuf {
    path.join("particles")
}
pub fn blueprint_dir(path: &Path) -> PathBuf {
    path.join("blueprint")
}
pub fn vault_dir(path: &Path) -> PathBuf {
    path.join("vault")
}

pub fn to_peer_id(kp: &Keypair) -> PeerId {
    PeerId::from(kp.public())
}
