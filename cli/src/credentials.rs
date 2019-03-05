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

use ethkey::Password;
use ethkey::Secret;
use ethstore::accounts_dir::{DiskKeyFileManager, KeyFileManager};
use ethstore::SafeAccount;
use failure::ResultExt;
use failure::{err_msg, Error};
use std::fs::File;

/// Authorization to call contract methods
#[derive(Debug, Clone)]
pub enum Credentials {
    No,
    Password(String),
    Secret(Secret),
}

impl Credentials {
    /// usage of secret key is priority
    pub fn get(secret: Option<Secret>, password: Option<String>) -> Credentials {
        match (secret, password) {
            (Some(secret), _) => Credentials::Secret(secret),
            (_, password) => Credentials::from_password(password),
        }
    }

    pub fn from_password(pass: Option<String>) -> Credentials {
        match pass {
            Some(p) => Credentials::Password(p.to_owned()),
            None => Credentials::No,
        }
    }
}

pub fn load_credentials(
    keystore: Option<String>,
    password: Option<String>,
    secret_key: Option<Secret>,
) -> Result<Credentials, Error> {
    match keystore {
        Some(keystore) => match password {
            Some(password) => load_keystore(keystore, password).map(Credentials::Secret),
            None => Err(err_msg("password is required for keystore")),
        },
        None => Ok(Credentials::get(secret_key, password)),
    }
}

pub fn load_keystore(path: String, password: String) -> Result<Secret, Error> {
    let keystore = File::open(path).context("can't open keystore file")?;
    let dkfm = DiskKeyFileManager {};
    let keystore: SafeAccount = dkfm
        .read(None, keystore)
        .map_err(|e| err_msg(e.to_string()))
        .context("can't parse keystore file")?;

    let password: Password = password.into();
    keystore
        .crypto
        .secret(&password)
        .map_err(|e| err_msg(e.to_string()))
        .context("can't parse secret from keystore file")
        .map_err(Into::into)
}
