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

#[macro_use]
extern crate lazy_static;

use std::error::Error;
use std::sync::Mutex;

use fluence::sdk::*;
use llamadb::tempdb::{ExecuteStatementResponse, TempDb};

use crate::signature::*;

mod signature;
#[cfg(test)]
mod tests;

/// Result for all possible Error types.
type GenResult<T> = ::std::result::Result<T, Box<Error>>;

lazy_static! {
    static ref DATABASE: Mutex<TempDb> = Mutex::new(TempDb::new());
}

/// Flag to toggle signature validation
static CHECK_SIGNATURE: bool = false;

/// Executes SQL and converts llamadb error to string.
#[invocation_handler]
fn main(input: String) -> String {
    let result = if CHECK_SIGNATURE {
        check_input(&input).and_then(run_query)
    } else {
        run_query(&input)
    };

    match result {
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
        ExecuteStatementResponse::Inserted(number) => format!("rows inserted: {}", number),
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
