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
