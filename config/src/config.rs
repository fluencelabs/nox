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

use libp2p_core::{identity::ed25519::Keypair, PeerId, PublicKey};
use std::path::{Path, PathBuf};

pub fn abs_path(path: PathBuf) -> PathBuf {
    match std::env::current_dir().ok() {
        Some(c) => c.join(path),
        None => path,
    }
}

pub fn workdir(path: &PathBuf) -> PathBuf {
    path.join("workdir")
}
pub fn modules_dir(path: &PathBuf) -> PathBuf {
    path.join("modules")
}
pub fn services_dir(path: &PathBuf) -> PathBuf {
    path.join("services")
}
pub fn blueprint_dir(path: &PathBuf) -> PathBuf {
    path.join("blueprint")
}

pub fn create_dirs<I>(dirs: I) -> Result<(), std::io::Error>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    for dir in dirs {
        std::fs::create_dir_all(dir)?;
    }

    Ok(())
}

pub fn to_peer_id(kp: &Keypair) -> PeerId {
    PeerId::from(PublicKey::Ed25519(kp.public()))
}
