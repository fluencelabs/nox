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

//
// Public functions for work with Llamadb.
//

/// Execute sql and returns result as string in the memory.
///
/// 1. Takes a pointer and length for a SQL string in memory, makes from them
///    Rust string.
/// 2. Processes the query for specified SQL string
/// 3. Returns a pointer to a result as a string in the memory.
/// 4. Deallocate memory from passed parameter
#[no_mangle]
pub unsafe fn do_query(ptr: *mut u8, len: usize) -> usize {
    // deallocation of parameter's memory will automatically appear in the end of the method
    let sql_str = deref_str(ptr, len);
    let db_response = match run_query(&sql_str) {
        Ok(response) => { response }
        Err(err_msg) => { "[Error] ".to_string() + err_msg.description() }
    };

    // return pointer to result in memory
    put_to_mem(db_response) as usize
}

//
// Public functions for memory management
//

/// Allocates memory area of specified size and returns address of the first
/// byte in the allocated memory area.
#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    alloc(size)
        .expect(format!("[Error] Allocation of {} bytes failed.", size).as_str())
}

unsafe fn alloc(size: usize) -> GenResult<NonNull<u8>> {
    let layout: Layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Global.alloc(layout).map_err(Into::into)
}

/// Deallocates memory area with first byte address = `ptr` and size = `size`.
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
    let statement = llamadb::sqlsyntax::parse_statement(sql_query);
    let mut db = DATABASE.lock()?;
    let result = db.execute_statement(statement)
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
        ExecuteStatementResponse::Explain(result) => {
            result.clone()
        }
    }
}

/// Writes Rust string into the memory directly as string length and byte array
/// (big-endian order). Written memory structure is:
///     | str_length: 8 BYTES | string_payload: str_length BYTES|
unsafe fn put_to_mem(str: String) -> *mut u8 {
    // converting string size to bytes in big-endian order
    let len_as_bytes: &[u8; 8] = mem::transmute(&str.len().to_be());

    let mut result: Vec<u8> = Vec::with_capacity(len_as_bytes.len() + str.len());
    result.write_all(len_as_bytes).unwrap();
    result.write_all(str.as_bytes()).unwrap();

    let result_ptr = allocate(result.len()).as_ptr();

    // writes bytes into memory byte-by-byte. Address of first byte will be == `ptr`
    for (idx, byte) in result.iter().enumerate() {
        std::ptr::write(result_ptr.offset(idx as isize), *byte);
    }

    result_ptr
}

/// Creates a public static reference to initialized LlamsDb instance.
lazy_static! {

    static ref DATABASE: Mutex<TempDb> = Mutex::new(TempDb::new());

}
