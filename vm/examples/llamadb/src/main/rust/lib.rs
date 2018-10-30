//! Module wrapper for Llamadb.
//!
//! It provides public methods for work with Llamadb and for `allocation` and
//! `deallocation` memory from a WASM host environment. Also contains functions
//! for reading from and writing strings to the raw memory.

#![feature(allocator_api)]
#![allow(dead_code)]

mod tests;

#[macro_use]
extern crate lazy_static;
extern crate fluence_sdk as fluence;
extern crate llamadb;

use llamadb::tempdb::ExecuteStatementResponse;
use llamadb::tempdb::TempDb;
use std::error::Error;
use std::num::NonZeroUsize;
use std::ptr::NonNull;
use std::sync::Mutex;

/// Result for all possible Error types.
type GenResult<T> = Result<T, Box<Error>>;

//
// Public functions for work with Llamadb.
//

/// Executes sql and returns the result as string in the memory.
///
/// 1. Takes a pointer and length for a the SQL string in memory, makes from them
///    Rust string.
/// 2. Processes the query for specified SQL string
/// 3. Returns a pointer to the result as a string in the memory.
/// 4. Deallocates memory from passed parameter
#[no_mangle]
pub fn do_query(ptr: *mut u8, len: usize) -> NonNull<u8> {
    // memory for the parameter will be deallocated when sql_str was dropped
    let sql_str: String = unsafe { fluence::memory::deref_str(ptr, len) };

    let db_response = match run_query(&sql_str) {
        Ok(response) => response,
        Err(err_msg) => format!("[Error] {}", err_msg),
    };

    unsafe {
        // return pointer to result in memory
        fluence::memory::write_str_to_mem(db_response)
            .expect("Putting result string to the memory was failed.")
    }
}

//
// Public functions for memory management
//

/// Allocates memory area of specified size and returns its address.
/// Used from the host environment for memory allocation for passed parameters.
#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    let non_zero_size =
        NonZeroUsize::new(size).expect("[Error] Allocation of zero bytes is not allowed.");
    fluence::memory::alloc(non_zero_size)
        .expect(format!("[Error] Allocation of {} bytes failed.", size).as_str())
}

/// Deallocates memory area for current memory pointer and size.
/// Used from the host environment for memory deallocation after reading results
/// of function from Wasm memory.
#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) -> () {
    let non_zero_size =
        NonZeroUsize::new(size).expect("[Error] Deallocation of zero bytes is not allowed.");
    fluence::memory::dealloc(ptr, non_zero_size)
        .expect(format!("[Error] Deallocate failed for prt={:?} size={}.", ptr, size).as_str())
}

/// Acquires lock, does query, releases lock, returns query result
fn run_query(sql_query: &str) -> GenResult<String> {
    let mut db = DATABASE.lock()?;
    let result = db
        .do_query(sql_query)
        .map(statement_to_string)
        .map_err(Into::into);
    result
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

/// Creates a public static reference to initialized LlamaDb instance.
lazy_static! {
    static ref DATABASE: Mutex<TempDb> = Mutex::new(TempDb::new());
}
