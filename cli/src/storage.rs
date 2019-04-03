/*
 * Copyright 2018 Fluence Labs Limited
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

use std::convert::Into;
use std::fs::{read_dir, File};
use std::io::prelude::*;
use std::path::PathBuf;

use base58::{FromBase58, FromBase58Error};
use failure::{err_msg, Error, ResultExt};
use reqwest::multipart::Form;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use web3::types::H256;

use crate::storage::Storage::{IPFS, SWARM, UNKNOWN};
use crate::utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Storage {
    SWARM,
    IPFS,
    UNKNOWN(u8),
}

impl Storage {
    pub fn from(n: u8) -> Storage {
        match n {
            0 => SWARM,
            1 => IPFS,
            u => UNKNOWN(u),
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            SWARM => 0,
            IPFS => 1,
            UNKNOWN(u) => *u,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct IpfsResponse {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Hash")]
    hash: String,
    #[serde(rename = "Size")]
    size: String,
}

/// uploads bytes to specified storage
pub fn upload_to_storage(
    storage_type: Storage,
    storage_url: &str,
    path: PathBuf,
) -> Result<H256, Error> {
    let hash = match storage_type {
        Storage::SWARM => {
            let hash = upload_code_to_swarm(storage_url, path)?;
            hash.parse().map_err(|e| {
                err_msg(format!(
                    "Swarm upload error: invalid hex returned {} {}",
                    hash, e
                ))
            })?
        }
        Storage::IPFS => upload_code_to_ipfs(storage_url, path)?,
        Storage::UNKNOWN(u) => Err(err_msg(format!("Unknown type of storage: {}", u)))?,
    };

    Ok(hash)
}

/// Uploads file of code to the Swarm
/// TODO add directories support
fn upload_code_to_swarm(url: &str, path: PathBuf) -> Result<String, Error> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let mut url = utils::parse_url(url)?;
    url.set_path("/bzz:/");

    let client = Client::new();
    let res = client
        .post(url)
        .body(buf)
        .header("Content-Type", "application/octet-stream")
        .send()
        .and_then(|mut r| r.text())
        .context("error uploading code to swarm")?;

    Ok(res)
}

/// Uploads files to IPFS
fn upload_code_to_ipfs(url: &str, path: PathBuf) -> Result<H256, Error> {
    let mut url = utils::parse_url(url)?;
    url.set_path("/api/v0/add");

    let path = path.as_path();

    let form = if path.is_dir() {
        url.set_query(Some("pin=true&recursive=true&wrap-with-directory=true"));
        let entry_iter = read_dir(path)?;
        entry_iter.fold(Ok(Form::new()), |form: Result<Form, Error>, entry| {
            let entry = entry?;
            let form = form?.file("path", entry.path())?;
            Ok(form)
        })?
    } else {
        url.set_query(Some("pin=true"));
        Form::new().file("path", path)?
    };

    let client = Client::new();
    let response = client
        .post(url)
        .multipart(form)
        .send()
        .and_then(|mut r| r.text())
        .context("Error uploading code to IPFS")?;

    let responses: Result<Vec<IpfsResponse>, Error> = response
        .as_str()
        .split_whitespace()
        .map(|str| {
            let resp: IpfsResponse = serde_json::from_str(str)?;
            Ok(resp)
        })
        .collect();

    let responses: Vec<IpfsResponse> = responses?;

    let base58_str = if responses.len() == 0 {
        Err(err_msg("Empty response"))?
    } else if responses.len() == 1 {
        responses[0].hash.as_str()
    } else {
        // directory hash is in response with empty name
        let dir_response: Option<&str> = responses
            .iter()
            .find(|r| r.name.is_empty())
            .map(|r| r.hash.as_str());
        dir_response.ok_or_else(|| err_msg("Multiple files uploaded, but no hash of directory."))?
    };

    utils::print_info_msg("IPFS file address", base58_str.to_owned());

    let bytes = base58_str
        .from_base58()
        .map_err(|err| match err {
            FromBase58Error::InvalidBase58Character(c, pos) => format!(
                "Base58 decoding error: Invalid character '{}' at position {}",
                c, pos
            ),
            FromBase58Error::InvalidBase58Length => format!(
                "Base58 decoding error: Invalid input length '{}'",
                base58_str.len()
            ),
        })
        .map_err(err_msg)
        .context("Error on base58 decoding")?;

    // drops first 2 bytes, because ipfs return multihash format
    Ok(bytes.as_slice()[2..].into())
}
