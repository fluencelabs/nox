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
use fluence_keypair::{key_pair::KeyFormat, KeyPair};
use fs_utils::create_dirs;

use log::info;
use std::path::PathBuf;
use std::str::FromStr;
use std::{
    fs::{self, File},
    io::{Error, ErrorKind, Write},
    path::Path,
};

/// Creates new key pair and store it in a `key_path` file.
fn create_new_key_pair(key_path: &Path, keypair_format: KeyFormat) -> Result<KeyPair, Error> {
    let parents = key_path.parent();
    if let Some(parent_path) = parents {
        create_dirs(&[&parent_path])?
    }

    let key_pair = KeyPair::generate(keypair_format);
    let encoded = bs58::encode(key_pair.to_vec()).into_string();

    let mut key_file = File::create(key_path).map_err(|err| {
        std::io::Error::new(
            err.kind(),
            format!("error creating keypair file {:?}: {:?}", key_path, err),
        )
    })?;
    key_file.write_all(encoded.as_bytes()).map_err(|err| {
        std::io::Error::new(
            err.kind(),
            format!("error writing keypair to {:?}: {:?}", key_path, err),
        )
    })?;

    Ok(key_pair)
}

fn read_key_pair_from_file(
    path: &Path,
    keypair_format: String,
) -> Result<KeyPair, Box<dyn std::error::Error>> {
    let base58 = fs::read_to_string(path).map_err(|e| {
        std::io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "Error reading keypair from {}: {}",
                path.to_str().unwrap_or("[PATH CONTAINS INVALID UNICODE]"),
                e
            ),
        )
    })?;

    decode_key_pair(base58.trim().to_string(), keypair_format)
}

pub fn decode_key_pair(
    base58: String,
    key_pair_format: String,
) -> Result<KeyPair, Box<dyn std::error::Error>> {
    let key_pair = bs58::decode(base58).into_vec()?;

    Ok(
        KeyPair::from_vec(key_pair, KeyFormat::from_str(&key_pair_format)?)
            .map_err(|e| Error::new(ErrorKind::InvalidInput, e.to_string()))?,
    )
}

/// Read the file with a secret key if it exists, generate a new key pair and write it to file if not.
pub fn load_key_pair(
    key_path: PathBuf,
    keypair_format: String,
    generate_on_absence: bool,
) -> Result<KeyPair, Box<dyn std::error::Error>> {
    if !key_path.exists() {
        return if generate_on_absence {
            info!("Generating a new key pair to {:?}", key_path);
            Ok(create_new_key_pair(
                &key_path,
                KeyFormat::from_str(&keypair_format)?,
            )?)
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "Path to secret key does not exist".to_string(),
            )
            .into())
        };
    }

    if !key_path.is_dir() {
        read_key_pair_from_file(&key_path, keypair_format)
    } else {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "Path to secret key is a directory.".to_string(),
        )
        .into())
    }
}
