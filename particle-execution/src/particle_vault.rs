/*
 * Copyright 2021 Fluence Labs Limited
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

use fluence_app_service::{MarineWASIConfig, ModuleDescriptor};
use std::io::ErrorKind;
use std::path;
use std::path::{Path, PathBuf};

use fluence_libp2p::PeerId;
use thiserror::Error;

use fs_utils::create_dir;

use crate::ParticleParams;
use crate::VaultError::WrongVault;
use VaultError::{CleanupVault, CreateVault, InitializeVault};

pub const VIRTUAL_PARTICLE_VAULT_PREFIX: &str = "/tmp/vault";

#[derive(Debug, Clone)]
pub struct ParticleVault {
    vault_dir: PathBuf,
}

impl ParticleVault {
    pub fn new(vault_dir: PathBuf) -> Self {
        Self { vault_dir }
    }

    pub fn real_worker_particle_vault(&self, peer_id: PeerId) -> PathBuf {
        self.vault_dir.join(peer_id.to_base58())
    }

    /// Returns Particle File Vault path on Nox's filesystem
    pub fn real_particle_vault(
        &self,
        peer_id: PeerId,
        particle_id: &str,
        particle_token: &str,
    ) -> PathBuf {
        self.real_worker_particle_vault(peer_id)
            .join(Self::format_particle_directory_name(
                particle_id,
                particle_token,
            ))
    }

    /// Returns Particle File Vault path on Marine's filesystem (ie how it would look like inside service)
    pub fn virtual_particle_vault(&self, particle_id: &str, particle_token: &str) -> PathBuf {
        Path::new(VIRTUAL_PARTICLE_VAULT_PREFIX).join(Self::format_particle_directory_name(
            particle_id,
            particle_token,
        ))
    }

    fn format_particle_directory_name(id: &str, token: &str) -> String {
        format!("{}-{}", id, token)
    }

    pub async fn initialize(&self) -> Result<(), VaultError> {
        tokio::fs::create_dir_all(&self.vault_dir)
            .await
            .map_err(InitializeVault)?;

        Ok(())
    }

    pub fn create(
        &self,
        current_peer_id: PeerId,
        particle_id: &str,
        particle_token: &str,
    ) -> Result<(), VaultError> {
        let path = self.real_particle_vault(current_peer_id, particle_id, particle_token);
        create_dir(path).map_err(CreateVault)?;

        Ok(())
    }

    pub fn put(
        &self,
        current_peer_id: PeerId,
        particle: &ParticleParams,
        filename: String,
        payload: &str,
    ) -> Result<PathBuf, VaultError> {
        let vault_dir = self.real_particle_vault(current_peer_id, &particle.id, &particle.token);
        // Note that we can't use `to_real_path` here since the target file cannot exist yet,
        // but `to_real_path` do path normalization which requires existence of the file to resolve
        // symlinks.
        let real_path = vault_dir.join(&filename);
        if let Some(parent_path) = real_path.parent() {
            create_dir(parent_path).map_err(CreateVault)?;
        }

        std::fs::write(real_path.clone(), payload.as_bytes())
            .map_err(|e| VaultError::WriteVault(e, filename))?;

        self.to_virtual_path(current_peer_id, particle, &real_path)
    }

    pub fn cat(
        &self,
        current_peer_id: PeerId,
        particle: &ParticleParams,
        virtual_path: &Path,
    ) -> Result<String, VaultError> {
        let real_path = self.to_real_path(current_peer_id, particle, virtual_path)?;

        let contents = std::fs::read_to_string(real_path)
            .map_err(|e| VaultError::ReadVault(e, virtual_path.to_path_buf()))?;

        Ok(contents)
    }

    pub fn cat_slice(
        &self,
        current_peer_id: PeerId,
        particle: &ParticleParams,
        virtual_path: &Path,
    ) -> Result<Vec<u8>, VaultError> {
        let real_path = self.to_real_path(current_peer_id, particle, virtual_path)?;
        std::fs::read(real_path).map_err(|e| VaultError::ReadVault(e, virtual_path.to_path_buf()))
    }

    pub async fn cleanup(
        &self,
        peer_id: PeerId,
        particle_id: &str,
        particle_token: &str,
    ) -> Result<(), VaultError> {
        let path = self.real_particle_vault(peer_id, particle_id, particle_token);
        match tokio::fs::remove_dir_all(&path).await {
            Ok(_) => Ok(()),
            // ignore NotFound
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(CleanupVault(err)),
        }?;

        Ok(())
    }

    /// Converts real path in `vault_dir` to virtual path with `VIRTUAL_PARTICLE_VAULT_PREFIX`.
    /// Virtual path looks like `/tmp/vault/<particle_id>/<path>`.
    fn to_virtual_path(
        &self,
        current_peer_id: PeerId,
        particle: &ParticleParams,
        path: &Path,
    ) -> Result<PathBuf, VaultError> {
        let virtual_prefix = self.virtual_particle_vault(&particle.id, &particle.token);
        let real_prefix = self.real_particle_vault(current_peer_id, &particle.id, &particle.token);
        let rest = path
            .strip_prefix(&real_prefix)
            .map_err(|e| WrongVault(Some(e), path.to_path_buf(), real_prefix))?;

        Ok(virtual_prefix.join(rest))
    }

    /// Converts virtual path with `VIRTUAL_PARTICLE_VAULT_PREFIX` to real path in `vault_dir`.
    /// Support full paths to the file in the vault starting this the prefix as well as relative paths
    /// inside the vault.
    /// For example, `some/file.txt` is valid and will be resolved to `REAL_PARTICLE_VAULT_PREFIX/some/file.txt`.
    fn to_real_path(
        &self,
        current_peer_id: PeerId,
        particle: &ParticleParams,
        virtual_path: &Path,
    ) -> Result<PathBuf, VaultError> {
        let rest = if virtual_path.has_root() {
            // If path starts with the `/` then we consider it a full path containing the virtual vault prefix
            let virtual_prefix = self.virtual_particle_vault(&particle.id, &particle.token);
            virtual_path.strip_prefix(&virtual_prefix).map_err(|e| {
                WrongVault(Some(e), virtual_path.to_path_buf(), virtual_prefix.clone())
            })?
        } else {
            // Otherwise we consider it a relative path inside the vault
            virtual_path
        };
        let real_prefix = self.real_particle_vault(current_peer_id, &particle.id, &particle.token);
        let real_path = real_prefix.join(rest);
        let resolved_path = real_path
            .canonicalize()
            .map_err(|e| VaultError::NotFound(e, virtual_path.to_path_buf()))?;
        // Check again after normalization that the path leads to the real particle vault
        if resolved_path.starts_with(&real_prefix) {
            Ok(resolved_path)
        } else {
            Err(WrongVault(None, resolved_path, real_prefix))
        }
    }

    /// Map `vault_dir/$current-peer-id` to `/tmp/vault` inside the service.
    /// Particle File Vaults will be available as `/tmp/vault/$particle_id`
    pub fn inject_vault(&self, current_peer_id: PeerId, module: &mut ModuleDescriptor) {
        let wasi = &mut module.config.wasi;
        if wasi.is_none() {
            *wasi = Some(MarineWASIConfig::default());
        }
        // SAFETY: set wasi to Some in the code above
        let wasi = wasi.as_mut().unwrap();

        let vault_dir = self.real_worker_particle_vault(current_peer_id);

        wasi.preopened_files.insert(vault_dir.clone());
        wasi.mapped_dirs
            .insert(VIRTUAL_PARTICLE_VAULT_PREFIX.into(), vault_dir);
    }
}

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Error creating vault_dir")]
    InitializeVault(#[source] std::io::Error),
    #[error("Error creating particle vault")]
    CreateVault(#[source] std::io::Error),
    #[error("Error cleaning up particle vault")]
    CleanupVault(#[source] std::io::Error),
    #[error("Incorrect vault path `{1}`: doesn't belong to vault (`{2}`)")]
    WrongVault(#[source] Option<path::StripPrefixError>, PathBuf, PathBuf),
    #[error("Incorrect vault  path `{1}`: doesn't exist")]
    NotFound(#[source] std::io::Error, PathBuf),
    #[error("Read vault failed for `{1}`: {0}")]
    ReadVault(#[source] std::io::Error, PathBuf),
    #[error("Write vault failed for filename `{1}`: {0}")]
    WriteVault(#[source] std::io::Error, String),
}
