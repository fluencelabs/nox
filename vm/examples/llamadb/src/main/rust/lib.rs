//! Module wrapper for Llamadb.
//!
//! It provides public methods for work with Llamadb and for `allocation` and
//! `deallocation` memory from a WASM host environment. Also contains functions
//! for reading from and writing strings to the raw memory.

#![feature(extern_prelude)]
#![feature(allocator_api)]
#![allow(dead_code)]

mod tests;

#[macro_use]
extern crate lazy_static;
extern crate llamadb;

use std::ptr::NonNull;
use std::alloc::{Alloc, Global, Layout};
use std::mem;
use std::error::Error;
use std::sync::Mutex;
use std::io::Write;
use llamadb::tempdb::TempDb;
use llamadb::tempdb::ExecuteStatementResponse;

/// Result for all possible Error types.
type GenResult<T> = Result<T, Box<Error>>;

pub const STR_LEN_BYTES: usize = 4;

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
pub fn do_query(ptr: *mut u8, len: usize) -> usize {

    let sql_str: String = unsafe { deref_str(ptr, len) };

    let db_response = match run_query(&sql_str) {
        Ok(response) => { response }
        Err(err_msg) => { format!("[Error] {}", err_msg) }
    };

    unsafe {
        // return pointer to result in memory
        put_to_mem(db_response) as usize
    }
}

//
// Public functions for memory management
//

/// Allocates memory area of specified size and returns its address.
/// Used from the host environment for memory allocation for passed parameters.
#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    alloc(size)
        .expect(format!("[Error] Allocation of {} bytes failed.", size).as_str())
}

unsafe fn alloc(size: usize) -> GenResult<NonNull<u8>> {
    let layout: Layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Global.alloc(layout).map_err(Into::into)
}

/// Deallocates memory area for current memory pointer and size.
/// Used from the host environment for memory deallocation after reading results
/// of function from Wasm memory.
#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) -> () {
    dealloc(ptr, size)
        .expect(format!("[Error] Deallocate failed for prt={:?} size={}.", ptr, size).as_str())
}

unsafe fn dealloc(ptr: NonNull<u8>, size: usize) -> GenResult<()> {
    let layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Ok(Global.dealloc(ptr, layout))
}

//
// Private functions with working with Strings and Memory.
//

/// Builds Rust string from the pointer and length.
unsafe fn deref_str(ptr: *mut u8, len: usize) -> String {
    String::from_raw_parts(ptr, len, len)
}

/// Acquires lock, does query, releases lock, returns query result
fn run_query(sql_query: &str) -> GenResult<String> {
    let mut db = DATABASE.lock()?;
    let result = db.do_query(sql_query)
        .map(statement_to_string)
        .map_err(Into::into);
    result
}

/// Converts query result to CSV String.
fn statement_to_string(statement: ExecuteStatementResponse) -> String {
    match statement {
        ExecuteStatementResponse::Created => {
            "table created".to_string()
        }
        ExecuteStatementResponse::Dropped => {
            "table was dropped".to_string()
        }
        ExecuteStatementResponse::Inserted(number) => {
            format!("rows inserted: {}", number)
        }
        ExecuteStatementResponse::Select { column_names, rows } => {
            let col_names = column_names.to_vec().join(", ") + "\n";
            let rows_as_str =
                rows.map(|row| {
                    row.iter()
                        .map(|elem| elem.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                }).collect::<Vec<String>>().join("\n");

            col_names + &rows_as_str
        }
        ExecuteStatementResponse::Deleted(number) => {
            format!("rows deleted: {}", number)
        }
        ExecuteStatementResponse::Explain(result) => {
            result
        }
        ExecuteStatementResponse::Updated(number) => {
            format!("rows updated: {}", number)
        }
    }
}

/// Writes Rust string into the memory directly as string length and byte array
/// (little-endian? order). Written memory structure is:
///     | str_length: 4 BYTES | string_payload: str_length BYTES|
unsafe fn put_to_mem(str: String) -> *mut u8 {
    // converting string size to bytes in little-endian order
    let len_as_bytes: [u8; STR_LEN_BYTES] = mem::transmute((str.len() as u32).to_le());
    let total_len = STR_LEN_BYTES
        .checked_add(str.len())
        .expect("usize overflow occurred");;

    let mut result: Vec<u8> = Vec::with_capacity(total_len);
    result.write_all(&len_as_bytes).unwrap();
    result.write_all(str.as_bytes()).unwrap();

    let result_ptr = allocate(total_len).as_ptr();
    std::ptr::copy_nonoverlapping(result.as_ptr(), result_ptr, total_len);

    result_ptr
}

/// Creates a public static reference to initialized LlamaDb instance.
lazy_static! {

    static ref DATABASE: Mutex<TempDb> = Mutex::new(TempDb::new());

}
