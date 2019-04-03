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

use failure::Error;

use clap::ArgMatches;
use clap::{value_t, App, AppSettings, Arg, SubCommand};
use derive_getters::Getters;
use web3::types::H256;

use crate::config::SetupConfig;
use crate::storage::Storage::{IPFS, SWARM};
use crate::storage::{upload_to_storage, Storage};
use crate::utils;
use std::path::{Path, PathBuf};

const CODE_PATH: &str = "code_path";
const STORAGE_URL: &str = "storage_url";
const IS_SWARM: &str = "swarm";

#[derive(Debug, Getters)]
pub struct Uploader {
    path: PathBuf,
    storage_url: String,
    storage_type: Storage,
}

impl Uploader {
    pub fn new(path: PathBuf, storage_url: String, storage_type: Storage) -> Uploader {
        Uploader {
            path,
            storage_url,
            storage_type,
        }
    }

    pub fn upload_code(self, show_progress: bool) -> Result<H256, Error> {
        let upload_to_storage_fn = || -> Result<H256, Error> {
            upload_to_storage(self.storage_type, &self.storage_url.as_str(), self.path)
        };

        let hash = if show_progress {
            utils::with_progress(
                "Uploading application code to storage...",
                "",
                "Application code uploaded.",
                upload_to_storage_fn,
            )?
        } else {
            upload_to_storage_fn()?
        };

        Ok(hash)
    }
}

pub fn parse(matches: &ArgMatches, config: SetupConfig) -> Result<Uploader, Error> {
    let path = value_t!(matches, CODE_PATH, String)?;
    let path = Path::new(path.as_str());

    let is_swarm = matches.is_present(IS_SWARM);

    let storage_type = if is_swarm { SWARM } else { IPFS };

    let storage_url = matches
        .value_of(STORAGE_URL)
        .map(|s| s.to_string())
        .unwrap_or(config.storage_url.clone());

    Ok(Uploader::new(path.to_owned(), storage_url, storage_type))
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    let args = &[
        Arg::with_name(CODE_PATH)
            .long(CODE_PATH)
            .short("c")
            .required(true)
            .takes_value(true)
            .help("Path to compiled `wasm` code")
            .display_order(0),
        Arg::with_name(STORAGE_URL)
            .long(STORAGE_URL)
            .short("w")
            .required(false)
            .takes_value(true)
            .help("Http address to storage node (IPFS by default)")
            .display_order(4),
        Arg::with_name(IS_SWARM)
            .long(IS_SWARM)
            .required(false)
            .help("Use Swarm to upload code")
            .display_order(5),
    ];

    SubCommand::with_name("upload")
        .about("Upload code to storage")
        .args(args)
        .setting(AppSettings::ArgRequiredElseHelp)
}
