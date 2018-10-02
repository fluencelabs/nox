//! Module wrapper for Llamadb.
//!
//! It provides public methods for work with Llamadb and for `allocation` and
//! `deallocation` memory from a WASM host environment. Also contains functions
//! for dereference string arguments for passing theirs into WASM functions.

#![feature(allocator_api)]

#[macro_use]
extern crate lazy_static;
extern crate llamadb;

use std::ptr::NonNull;
use std::alloc::{Alloc, Global, Layout};
use std::mem;
use std::error::Error;

/// Result for all possible Error types.
type GenResult<T> = Result<T, Box<Error>>;

//
// Public functions for work with Llamadb.
//

//
// Public functions for memory management
//

/// Allocates memory area of specified size and returns address of the first
/// byte in the allocated memory area.
#[no_mangle]
pub unsafe fn alloc(size: usize) -> NonNull<u8> {
    allocate(size)
        .expect(format!("[Error] Allocation of {} bytes failed.", size).as_str())
}

unsafe fn allocate(size: usize) -> GenResult<NonNull<u8>> {
    let layout: Layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Global.alloc(layout)
}

/// Deallocates memory area with first byte address = `ptr` and size = `size`.
#[no_mangle]
pub unsafe fn dealloc(ptr: NonNull<u8>, size: usize) -> () {
    deallocate(ptr, size)
        .expect(format!("[Error] Deallocate failed for prt={:?} size={}.", ptr, size).as_str())
}

unsafe fn deallocate(ptr: NonNull<u8>, size: usize) -> GenResult<()> {
    let layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Ok(Global.dealloc(ptr, layout))
}

//
// Private functions with working with Strings and Memory.
//