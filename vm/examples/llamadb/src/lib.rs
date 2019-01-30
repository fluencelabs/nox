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
//! Provides the public method (`invoke`) for work with Llamadb and some public service
//! methods (`allocation`, `deallocation`, `init_logger`).

#![feature(allocator_api)]
#![allow(dead_code)]

#[cfg(test)]
mod tests;

#[macro_use]
extern crate lazy_static;

use llamadb::tempdb::{ExecuteStatementResponse, TempDb};
use log::{error, info};
use std::error::Error;
use std::ptr::NonNull;
use std::sync::Mutex;

/// Result for all possible Error types.
type GenResult<T> = ::std::result::Result<T, Box<Error>>;

//
// FFI for interaction with Llamadb module.
//

/// Executes SQL and returns a pointer to result as a string in the memory.
///
/// 1. Makes rust String from given pointer and length
/// 2. Processes the resulted string as a SQL query
/// 3. Returns a pointer to the result as a string in the memory
/// 4. Deallocates memory occupied by passed parameter
#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> NonNull<u8> {
    info!("invoke starts with ptr={:?}, len={}", ptr, len);

    // memory for the parameter will be deallocated when sql_str was dropped
    let sql_str = fluence::memory::read_input_from_mem(ptr, len);
    let sql_str = String::from_utf8(sql_str).unwrap();

    let db_response = match run_query(&sql_str) {
        Ok(response) => response,
        Err(err_msg) => format!("[Error] {}", err_msg),
    };

    info!("llamadb do_query ends with result={:?}", db_response);
    // return pointer to result in memory
    fluence::memory::write_result_to_mem(db_response.as_bytes())
        .unwrap_or_else(|_| log_and_panic("Putting result string to the memory was failed.".into()))
}

fn log_and_panic(msg: String) -> ! {
    error!("{}", msg);
    panic!(msg);
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

lazy_static! {
    static ref DATABASE: Mutex<TempDb> = Mutex::new(TempDb::new());
}
