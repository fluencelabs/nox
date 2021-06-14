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

use rand::Rng;
use std::fmt::Debug;
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

pub fn create_dirs<Item>(dirs: &[Item]) -> Result<(), std::io::Error>
where
    Item: AsRef<Path> + Debug,
{
    for dir in dirs {
        create_dir(dir)?;
    }

    Ok(())
}

pub fn create_dir<P: AsRef<Path> + Debug>(dir: P) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(&dir)
        .map_err(|err| std::io::Error::new(err.kind(), format!("{:?}: {:?}", err, dir)))
}

pub fn remove_dir(dir: &Path) -> Result<(), std::io::Error> {
    std::fs::remove_dir_all(&dir)
        .map_err(|err| std::io::Error::new(err.kind(), format!("{:?}: {:?}", err, dir)))
}
