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

use log::info;
use std::fs;
use std::fs::File;
use std::io::{Error, ErrorKind};
use std::io::{Read, Write};
use std::path::Path;
use trust_graph::key_pair::KeyPair;

/// Creates new key pair and store it in a `key_path` file.
fn create_new_key_pair(key_path: &Path) -> Result<KeyPair, Error> {
    let parents = key_path.parent();
    if let Some(parent_path) = parents {
        fs::create_dir_all(parent_path)?
    }

    let key_pair = KeyPair::generate();
    let encoded = key_pair.encode();

    let mut key_file = File::create(key_path)?;
    key_file.write_all(&encoded)?;

    Ok(key_pair)
}

fn read_key_pair_from_file(key_path: &Path) -> Result<KeyPair, Error> {
    let mut file = File::open(key_path)?;

    let mut key_path_bytes = [0; 64];

    file.read_exact(&mut key_path_bytes)?;

    let key_pair = KeyPair::decode(&mut key_path_bytes)
        .map_err(|e| Error::new(ErrorKind::InvalidInput, e.to_string()))?;

    Ok(key_pair)
}

/// Read the file with a secret key if it exists, generate a new key pair and write it to file if not.
pub fn load_or_create_key_pair(key_path: &str) -> Result<KeyPair, Error> {
    let key_path = Path::new(key_path);

    if !key_path.exists() {
        info!("generating a new key pair");
        return create_new_key_pair(key_path);
    }

    if !key_path.is_dir() {
        return read_key_pair_from_file(key_path);
    }

    Err(Error::new(
        ErrorKind::InvalidInput,
        "Path to secret key is a directory.".to_string(),
    ))
}
