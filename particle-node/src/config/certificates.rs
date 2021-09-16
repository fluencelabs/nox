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

use fluence_identity::KeyPair;
use trust_graph::Certificate;

use eyre::{eyre, WrapErr};
use log::info;
use std::{
    fs::{self, create_dir, File},
    io::{Error, Write},
    path::Path,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Loads all certificates from a disk. Creates a root certificate for key pair if there is no one.
pub fn init(certificate_dir: &Path, key_pair: &KeyPair) -> eyre::Result<Vec<Certificate>> {
    let mut certs = load_certificates(certificate_dir)
        .wrap_err_with(|| format!("failed to load root certificates on init from {:?}", certificate_dir))?;

    let public_key = key_pair.public();

    let root_cert = certs.iter().find(|c| c.chain[0].issued_for == public_key);

    // Creates and stores a new root certificate if needed.
    if root_cert.is_none() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let expires_at = now.checked_add(Duration::new(60 * 60 * 24 * 365, 0)).unwrap();
        let root_cert = store_root_certificate(certificate_dir, key_pair, expires_at, now)
            .context("Failed to store root certificates on init")?;
        certs.push(root_cert);
    }

    Ok(certs)
}

/// Reads all files in `cert_dir` as certificates.
/// Throw an error, if one of the files has an incorrect format.
pub fn load_certificates(cert_dir: &Path) -> eyre::Result<Vec<Certificate>> {
    // cold start, if there is no directory, create a new one
    if !cert_dir.exists() {
        create_dir(&cert_dir).wrap_err_with(|| format!("failed to create cert_dir {:?}", cert_dir))?;
    }

    if cert_dir.is_file() {
        return Err(eyre!("Path to certificates is not a directory."));
    }

    let mut certs = Vec::new();

    for entry in fs::read_dir(&cert_dir).wrap_err_with(|| format!("failed to read cert_dir {:?}", cert_dir))? {
        let entry = entry.wrap_err("read_dir entry failed")?;
        let path = entry.path();

        // ignore sub directories
        if !path.is_dir() {
            let str_cert = fs::read_to_string(&path).wrap_err_with(|| format!("can't read {:?}", path))?;
            let cert =
                Certificate::from_str(str_cert.as_str()).map_err(|e| eyre!("error parsing certificate: {:#?}", e))?;
            certs.push(cert);
        }
    }

    Ok(certs)
}

pub fn store_root_certificate(
    cert_dir: &Path,
    key_pair: &KeyPair,
    expires_at: Duration,
    issued_at: Duration,
) -> Result<Certificate, Error> {
    info!("storing new certificate for the key pair");
    let cert: Certificate = Certificate::issue_root(key_pair, key_pair.public(), expires_at, issued_at);

    let root_cert_path = cert_dir.join(Path::new("root.cert"));

    let mut file = File::create(root_cert_path)?;

    file.write_all(cert.to_string().as_bytes())?;

    Ok(cert)
}
