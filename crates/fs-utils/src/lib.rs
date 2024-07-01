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

#![recursion_limit = "512"]
#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

use eyre::{eyre, Context};
use futures_util::StreamExt;
use std::fmt::Debug;
use std::fs;
use std::fs::{DirBuilder, Permissions};
use std::future::Future;
use std::io::ErrorKind;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::thread::available_parallelism;
use thiserror::Error;
use tokio::fs::DirEntry;
use tokio_stream::wrappers::ReadDirStream;

// default bound on the number of computations it can perform simultaneously
const DEFAULT_PARALLELISM: usize = 2;

type Err = Box<dyn std::error::Error + Send + Sync>;
type Res<T> = Result<T, Err>;

pub fn to_abs_path(path: PathBuf) -> PathBuf {
    match std::env::current_dir().ok() {
        Some(c) => c.join(path),
        None => path,
    }
}

pub fn create_dirs<Item>(dirs: &[Item]) -> Result<(), std::io::Error>
where
    Item: AsRef<Path> + Debug,
{
    for dir in dirs {
        create_dir(dir)?;
    }

    Ok(())
}

pub fn set_write_only(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = Permissions::from_mode(0o733);
    std::fs::set_permissions(path, permissions).map_err(|err| {
        std::io::Error::new(
            err.kind(),
            format!("error making path write only (733) {err:?}: {path:?}"),
        )
    })
}

pub fn create_dir<P: AsRef<Path> + Debug>(dir: P) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(&dir)
        .map_err(|err| std::io::Error::new(err.kind(), format!("{err:?}: {dir:?}")))
}

// TODO: this is a hack, remove after fix in marine. mask 300 doesn't work on macos
cfg_if::cfg_if! {
     if #[cfg(target_os = "macos")] {
        const WRITE_ONLY_MASK: u32 = 0o700;
    } else {
        const WRITE_ONLY_MASK: u32 = 0o300;
    }
}

pub fn create_dir_write_only<P: AsRef<Path> + Debug>(dir: P) -> Result<(), std::io::Error> {
    DirBuilder::new()
        .recursive(true)
        .mode(WRITE_ONLY_MASK)
        .create(&dir)
        .map_err(|err| std::io::Error::new(err.kind(), format!("{err:?}: {dir:?}")))
}

pub fn remove_dirs<Item>(dirs: &[Item]) -> Result<(), std::io::Error>
where
    Item: AsRef<Path> + Debug,
{
    for dir in dirs {
        remove_dir(dir.as_ref())?;
    }

    Ok(())
}

pub fn remove_dir(dir: &Path) -> Result<(), std::io::Error> {
    match std::fs::remove_dir_all(dir) {
        Ok(_) => Ok(()),
        // ignore NotFound
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(std::io::Error::new(
            err.kind(),
            format!("error removing directory {dir:?}: {err:?}"),
        )),
    }
}

pub fn remove_file(file: &Path) -> Result<(), std::io::Error> {
    match std::fs::remove_file(file) {
        Ok(_) => Ok(()),
        // ignore NotFound
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(std::io::Error::new(
            err.kind(),
            format!("error removing file {file:?}: {err:?}"),
        )),
    }
}

pub fn file_stem(path: impl AsRef<Path>) -> eyre::Result<String> {
    let path = path.as_ref();
    Ok(path
        .file_stem()
        .ok_or_else(|| eyre!("invalid path (no file name): {:?}", path))?
        .to_str()
        .ok_or_else(|| eyre!("path {:?} contain non-UTF-8 character", path))?
        .to_string())
}

pub fn file_name(path: impl AsRef<Path>) -> eyre::Result<String> {
    let path = path.as_ref();
    Ok(path
        .file_name()
        .ok_or_else(|| eyre!("invalid path (no file name): {:?}", path))?
        .to_str()
        .ok_or_else(|| eyre!("path {:?} contain non-UTF-8 character", path))?
        .to_string())
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> eyre::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }

    Ok(())
}

pub fn canonicalize(path: impl AsRef<Path>) -> eyre::Result<PathBuf> {
    path.as_ref().canonicalize().context(format!(
        "Error canonicalizing path '{}'",
        path.as_ref().display()
    ))
}

/// List files in directory
pub fn list_files(dir: &Path) -> Option<impl Iterator<Item = PathBuf>> {
    let dir = std::fs::read_dir(dir).ok()?;
    Some(dir.filter_map(|p| p.ok()?.path().into()))
}

#[derive(Debug, Error)]
pub enum LoadDataError {
    #[error("Error creating directory for data {path:?}: {err}")]
    CreateDir {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error reading persisted data from {path:?}: {err}")]
    ReadPersistedData {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error deserializing data from {path:?}: {err}")]
    DeserializeData {
        path: PathBuf,
        #[source]
        err: Err,
    },
}

/// Load some data from disk in parallel
pub async fn load_persisted_data<T>(
    data_dir: &Path,
    filter: fn(&Path) -> bool,
    de: fn(&[u8]) -> Res<T>,
) -> Result<Vec<(T, PathBuf)>, LoadDataError> {
    let parallelism = available_parallelism()
        .map(|x| x.get())
        .unwrap_or(DEFAULT_PARALLELISM);
    let list_files = tokio::fs::read_dir(data_dir).await.ok();
    let entries = match list_files {
        Some(entries) => {
            let entries: Vec<(T, PathBuf)> = ReadDirStream::new(entries)
                .filter_map(|res| async move {
                    match res {
                        Ok(entry) => process_dir_entry(entry, filter, de),
                        Err(err) => {
                            tracing::warn!("Could not read data directory: {err}");
                            None
                        }
                    }
                })
                .buffer_unordered(parallelism)
                //collect only loaded data and unwrap Option
                .filter_map(|e| async { e })
                .collect()
                .await;

            entries
        }
        None => {
            // Attempt to create directory
            tokio::fs::create_dir_all(data_dir)
                .await
                .map_err(|err| LoadDataError::CreateDir {
                    path: data_dir.to_path_buf(),
                    err,
                })?;
            vec![]
        }
    };

    Ok(entries)
}

fn process_dir_entry<T>(
    entry: DirEntry,
    filter: fn(&Path) -> bool,
    de: fn(&[u8]) -> Res<T>,
) -> Option<impl Future<Output = Option<(T, PathBuf)>> + Sized> {
    let path = entry.path();
    if filter(path.as_path()) {
        let task = async move {
            let res = parse_persisted_data(path.as_path(), de).await;
            if let Err(err) = &res {
                tracing::warn!("Failed to load data: {err}")
            }
            res.ok().map(|data| (data, path))
        };

        Some(task)
    } else {
        None
    }
}

async fn parse_persisted_data<T>(file: &Path, de: fn(&[u8]) -> Res<T>) -> Result<T, LoadDataError> {
    let bytes = tokio::fs::read(file)
        .await
        .map_err(|err| LoadDataError::ReadPersistedData {
            err,
            path: file.to_path_buf(),
        })?;

    de(bytes.as_slice()).map_err(|err| LoadDataError::DeserializeData {
        err,
        path: file.to_path_buf(),
    })
}
