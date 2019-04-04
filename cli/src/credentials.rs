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

use std::fs::File;

use ethkey::Password;
use ethkey::Secret;
use ethkey::{public_to_address, KeyPair};
use ethstore::accounts_dir::{DiskKeyFileManager, KeyFileManager};
use ethstore::SafeAccount;
use failure::ResultExt;
use failure::{err_msg, Error};
use web3::types::Address;

/// Authorization to call contract methods
#[derive(Debug, Clone)]
pub enum Credentials {
    No,
    Secret(Secret),
    Keystore {
        secret: Secret,
        path: String,
        password: String,
    },
}

impl Credentials {
    pub fn from_secret(secret: Secret) -> Result<Credentials, Error> {
        secret
            .check_validity()
            .map(|_| Credentials::Secret(secret))
            .context("Secret isn't valid")
            .map_err(Into::into)
    }

    pub fn to_address(&self) -> Option<Address> {
        match self {
            &Credentials::Secret(ref s) | &Credentials::Keystore { secret: ref s, path: _, password: _ } => {
                KeyPair::from_secret(s.clone())
                    .ok()
                    .map(|s| public_to_address(s.public()))
            },
            _ => None
        }
    }

    pub fn load(
        keystore: Option<String>,
        password: Option<String>,
        secret_key: Option<Secret>,
    ) -> Result<Credentials, Error> {
        match keystore {
            Some(path) => {
                let password = password.ok_or(err_msg("password is required for keystore"))?;
                Credentials::load_keystore(path, password)
            }
            _ if secret_key.is_some() => {
                let secret = secret_key.expect("Secret key is None. This shouldn't happen.");
                Credentials::from_secret(secret)
            }
            _ => Ok(Credentials::No),
        }
    }

    pub fn load_keystore(path: String, password: String) -> Result<Credentials, Error> {
        let keystore = File::open(&path).context("can't open keystore file")?;
        let dkfm = DiskKeyFileManager {};
        let keystore: SafeAccount = dkfm
            .read(None, keystore)
            .map_err(|e| err_msg(e.to_string()))
            .context("can't parse the keystore file")?;
        let password: Password = password.into();
        let secret = keystore
            .crypto
            .secret(&password)
            .map_err(|e| err_msg(e.to_string()))
            .context("can't parse secret from the keystore file")?;
        secret.check_validity()?;

        let password = password.as_str().to_string();

        Ok(Credentials::Keystore {
            secret,
            path,
            password,
        })
    }
}
