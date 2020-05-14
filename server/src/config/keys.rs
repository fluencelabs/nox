/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use log::info;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::{Error, ErrorKind};
use std::path::Path;
use trust_graph::KeyPair;

/// Creates new key pair and store it in a `key_path` file.
fn create_new_key_pair(key_path: &Path) -> Result<KeyPair, Error> {
    let parents = key_path.parent();
    if let Some(parent_path) = parents {
        fs::create_dir_all(parent_path)?
    }

    let key_pair = KeyPair::generate();
    let encoded = bs58::encode(key_pair.encode().as_ref()).into_string();

    let mut key_file = File::create(key_path)?;
    key_file.write_all(encoded.as_bytes())?;

    Ok(key_pair)
}

fn read_key_pair_from_file(path: &Path) -> Result<KeyPair, Box<dyn std::error::Error>> {
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

    decode_key_pair(base58.trim().to_string())
}

pub fn decode_key_pair(base58: String) -> Result<KeyPair, Box<dyn std::error::Error>> {
    let mut key_pair = bs58::decode(base58).into_vec()?;

    Ok(KeyPair::decode(key_pair.as_mut())
        .map_err(|e| Error::new(ErrorKind::InvalidInput, e.to_string()))?)
}

/// Read the file with a secret key if it exists, generate a new key pair and write it to file if not.
pub fn load_or_create_key_pair(path: &str) -> Result<KeyPair, Box<dyn std::error::Error>> {
    let key_path = Path::new(path);

    if !key_path.exists() {
        info!("generating a new key pair");
        return Ok(create_new_key_pair(key_path)?);
    }

    if !key_path.is_dir() {
        return Ok(read_key_pair_from_file(key_path)?);
    }

    Err(Error::new(
        ErrorKind::InvalidInput,
        "Path to secret key is a directory.".to_string(),
    )
    .into())
}
