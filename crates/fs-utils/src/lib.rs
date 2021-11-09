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

use eyre::eyre;
use rand::Rng;
use std::fmt::Debug;
use std::fs;
use std::fs::Permissions;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub fn to_abs_path(path: PathBuf) -> PathBuf {
    match std::env::current_dir().ok() {
        Some(c) => c.join(path),
        None => path,
    }
}

pub fn make_tmp_dir() -> PathBuf {
    use rand::distributions::Alphanumeric;

    let mut tmp = std::env::temp_dir();
    tmp.push("fluence_test/");
    let dir: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(16)
        .collect();
    tmp.push(dir);

    create_dir(&tmp).expect("create tmp dir");

    tmp
}

pub fn make_tmp_dir_peer_id(peer_id: String) -> PathBuf {
    let mut tmp = std::env::temp_dir();
    tmp.push("fluence_test/");
    tmp.push(peer_id);

    create_dir(&tmp).expect("create tmp dir");

    tmp
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
    std::fs::set_permissions(&path, permissions).map_err(|err| {
        std::io::Error::new(
            err.kind(),
            format!("error making path write only (733) {:?}: {:?}", err, path),
        )
    })
}

pub fn create_dir<P: AsRef<Path> + Debug>(dir: P) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(&dir)
        .map_err(|err| std::io::Error::new(err.kind(), format!("{:?}: {:?}", err, dir)))
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
            format!("error removing directory {:?}: {:?}", dir, err),
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
            format!("error removing file {:?}: {:?}", file, err),
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
