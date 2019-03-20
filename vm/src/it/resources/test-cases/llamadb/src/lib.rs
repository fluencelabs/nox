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

//! Wrapper for Llamadb (a test for Fluence network).
//!
//! Provides the FFI (`main`) for interact with Llamadb.

use fluence::sdk::*;

#[cfg(test)]
mod tests;

#[macro_use]
extern crate lazy_static;

use llamadb::tempdb::{ExecuteStatementResponse, TempDb};
use std::error::Error;
use std::sync::Mutex;
use secp256k1::{Secp256k1, Message, SecretKey, PublicKey};


/// Result for all possible Error types.
type GenResult<T> = ::std::result::Result<T, Box<Error>>;

lazy_static! {
    static ref DATABASE: Mutex<TempDb> = Mutex::new(TempDb::new());
    static ref PK: PublicKey = get_pk();
}

// 64 + 1 byte prefix, should it be just 64?
static PUBLIC_KEY: [u8; 65] = [
    0x04, 0xfb, 0x6e, 0x27, 0x79, 0x77, 0xf4, 0x67,
    0x61, 0x8a, 0xde, 0x83, 0xf7, 0x50, 0x5b, 0x6f,
    0x44, 0x8b, 0xed, 0x40, 0xff, 0x10, 0x6d, 0xfd,
    0xde, 0x56, 0xde, 0x82, 0xfb, 0x14, 0xc7, 0x8a,
    0x53, 0x07, 0x57, 0x8e, 0x60, 0x91, 0x90, 0xd6,
    0x5f, 0xc6, 0x39, 0x61, 0x97, 0x0c, 0xf1, 0x48,
    0x62, 0x3f, 0x3d, 0xc8, 0xfc, 0x8e, 0x33, 0x17,
    0x7a, 0xa0, 0x5a, 0xdb, 0x4b, 0x78, 0x10, 0x28, 0x2b
];

fn get_pk() -> PublicKey {
    PublicKey::from_slice(&PUBLIC_KEY).expect("Invalid public key")
}

fn check_signature(signature: &str) -> bool {
    false
}

/// Executes SQL and converts llamadb error to string.
#[invocation_handler]
fn main(sql_str: String) -> String {
    match run_query(&sql_str) {
        Ok(response) => response,
        Err(err_msg) => format!("[Error] {}", err_msg),
    }
}

/// Acquires lock, does query, releases lock, returns query result.
fn run_query(sql_query: &str) -> GenResult<String> {
    let mut db = DATABASE.lock()?;
    db.do_query(sql_query)
        .map(statement_to_string)
        .map_err(Into::into)
}

/// Converts query result to CSV String.
fn statement_to_string(statement: ExecuteStatementResponse) -> String {
    match statement {
        ExecuteStatementResponse::Created => "table created".to_string(),
        ExecuteStatementResponse::Dropped => "table was dropped".to_string(),
        ExecuteStatementResponse::Inserted(number) => format!("rows inserted: {} {:?}", number, *PK),
        ExecuteStatementResponse::Select { column_names, rows } => {
            let col_names = column_names.to_vec().join(", ") + "\n";
            let rows_as_str = rows
                .map(|row| {
                    row.iter()
                        .map(|elem| elem.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                })
                .collect::<Vec<String>>()
                .join("\n");

            col_names + &rows_as_str
        }
        ExecuteStatementResponse::Deleted(number) => format!("rows deleted: {}", number),
        ExecuteStatementResponse::Explain(result) => result,
        ExecuteStatementResponse::Updated(number) => format!("rows updated: {}", number),
    }
}
