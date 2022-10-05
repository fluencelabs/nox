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
use std::path::PathBuf;
use std::str::FromStr;
use std::{
    fs::{self, File},
    io::{Error, ErrorKind, Write},
    path::Path,
};

use eyre::eyre;
use fluence_keypair::{key_pair::KeyFormat, KeyPair};
use log::info;

use fs_utils::create_dirs;

/// Creates new key pair and store its secret key in a `key_path` file.
fn create_new_key_pair(key_path: &Path, key_format: KeyFormat) -> Result<KeyPair, Error> {
    let parents = key_path.parent();
    if let Some(parent_path) = parents {
        create_dirs(&[&parent_path])?
    }

    let key_pair = KeyPair::generate(key_format);
    let secret_key = key_pair
        .secret()
        .expect("error getting secret key from keypair");
    let encoded = base64::encode(secret_key.to_vec());

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

pub fn decode_key(key_string: String, key_format: String) -> eyre::Result<KeyPair> {
    let key_string = key_string.trim();

    fn validate_length(v: Vec<u8>) -> eyre::Result<Vec<u8>> {
        if v.len() == 64 || v.len() <= 32 {
            Ok(v)
        } else {
            Err(eyre!("keypair length must be 64, was {}", v.len()))
        }
    }

    let bytes_from_base64 = base64::decode(key_string)
        .map_err(|err| eyre!("base64 decoding failed: {}", err))
        .and_then(validate_length);

    let bytes = if bytes_from_base64.is_err() {
        let bytes_from_base58 = bs58::decode(key_string)
            .into_vec()
            .map_err(|err| eyre!("base58 decoding failed: {}", err))
            .and_then(validate_length);
        if bytes_from_base58.is_err() {
            return Err(eyre!(
                "Tried to decode secret key as base64 and as base58 from string {}, but failed.\nbase64 decoding error:{}\nbase58 decoding error:{}",
                key_string, bytes_from_base64.err().unwrap(), bytes_from_base58.err().unwrap()
            ));
        } else {
            bytes_from_base58.unwrap()
        }
    } else {
        bytes_from_base64.unwrap()
    };

    // It seems that secp256k1 secret key length can be less than 32 byte
    // so for everything longer than 32 we try decode keypair, and shorter – decode secret key
    if bytes.len() > 32 {
        decode_key_pair(bytes, key_format)
    } else {
        decode_secret_key(bytes, key_format)
    }
}

/// read base64 secret key from file and generate key pair from it
fn read_secret_key_from_file(key_path: &Path, key_format: String) -> eyre::Result<KeyPair> {
    let key_string = fs::read_to_string(key_path).map_err(|e| {
        eyre!(
            "Error reading secret key from {}: {}",
            key_path.display(),
            e
        )
    })?;

    decode_key(key_string, key_format).map_err(|err| {
        eyre!(
            "failed to decode key at path {}: {}",
            key_path.display(),
            err
        )
    })
}

pub fn decode_key_pair(key_pair: Vec<u8>, key_format: String) -> eyre::Result<KeyPair> {
    Ok(
        KeyPair::from_vec(key_pair, KeyFormat::from_str(&key_format)?)
            .map_err(|e| eyre!("Error decoding keypair of format {}: {:?}", key_format, e))?,
    )
}

pub fn decode_secret_key(secret_key: Vec<u8>, key_format: String) -> eyre::Result<KeyPair> {
    Ok(
        KeyPair::from_secret_key(secret_key, KeyFormat::from_str(&key_format)?).map_err(|e| {
            eyre!(
                "Error decoding secret key of format {}: {:?}",
                key_format,
                e
            )
        })?,
    )
}

/// Read the file with a secret key if it exists, generate a new key pair and write it to file if not.
pub fn load_key(
    key_path: PathBuf,
    key_format: String,
    generate_on_absence: bool,
) -> eyre::Result<KeyPair> {
    if !key_path.exists() {
        return if generate_on_absence {
            info!("Generating a new key to {:?}", key_path);
            Ok(create_new_key_pair(
                &key_path,
                KeyFormat::from_str(&key_format)?,
            )?)
        } else {
            Err(eyre!(
                "Path to secret key does not exist {}",
                key_path.display(),
            ))
        };
    }

    if !key_path.is_dir() {
        read_secret_key_from_file(&key_path, key_format)
    } else {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "Path to secret key is a directory.".to_string(),
        )
        .into())
    }
}
