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

use crate::utils;
use failure::{Error, ResultExt, err_msg};
use reqwest::Client;
use web3::types::H256;
use hyper::rt::Future;
use ipfs_api::IpfsClient;
use std::borrow::ToOwned;
use std::io::Cursor;
use futures::{future, Future};

#[derive(Debug)]
pub enum Storage {
    SWARM,
    IPFS,
}

pub fn upload_to_storage(
    storage_type: &Storage,
    storage_url: &str,
    bytes: &[u8],
) -> Result<H256, Error> {
    println!("hi11111");
    let hash = match storage_type {
        Storage::SWARM => upload_code_to_swarm(storage_url, bytes)?,
        Storage::IPFS => upload_code_to_ipfs(storage_url, bytes)?,
    };

    println!("result = {:?}", hash);

    let hash = hash.parse().map_err(err_msg).context("Error on hex parsing")?;
    Ok(hash)
}

/// Uploads bytes of code to the Swarm
fn upload_code_to_swarm(url: &str, bytes: &[u8]) -> Result<String, Error> {
    let mut url = utils::parse_url(url)?;
    url.set_path("/bzz:/");

    let client = Client::new();
    let res = client
        .post(url)
        .body(bytes.to_vec())
        .header("Content-Type", "application/octet-stream")
        .send()
        .and_then(|mut r| r.text())
        .context("error uploading code to swarm")?;

    Ok(res)
}

fn upload_code_to_ipfs(url: &str, bytes: &[u8]) -> Result<String, Error> {
    let mut url = utils::parse_url(url)?;
    let host = format!("{:?}", url.host().unwrap()).as_str();

    let data = Cursor::new(bytes);

    let client = IpfsClient::new(host, url.port().unwrap())?;
    let req = client.add(data);


    let resp = hyper::rt::run(req).wait();


    Ok("".to_owned())
}
