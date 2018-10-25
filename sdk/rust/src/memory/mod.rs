//! Basic functions for dealing with WASM memory.
//!
//! This module contains functions for initializing and manipulating memory.

pub mod errors;

use std::ptr::NonNull;
use std::alloc::{Alloc, Global, Layout};
use std::mem;
use std::io::Write;
use self::errors::MemError;


/// Result for all possible Error types.
pub type MemResult<T> = Result<T, MemError>;

/// Allocates memory area of specified size and returns its address.
pub unsafe fn alloc<'cause>(size: usize) -> MemResult<NonNull<u8>> {
    let layout: Layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Global.alloc(layout).map_err(Into::into)
}

/// Deallocates memory area for current memory pointer and size.
pub unsafe fn dealloc(ptr: NonNull<u8>, size: usize) -> MemResult<()> {
    let layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Ok(Global.dealloc(ptr, layout))
}

pub const STR_LEN_BYTES: usize = 4;

/// Writes Rust string into the memory directly as string length and byte array.
/// Written memory structure is:
/// ```
///     | str_length: 4 BYTES (little-endian) | string_payload: str_length BYTES |
/// ```
pub unsafe fn put_to_mem(str: String) -> MemResult<*mut u8> {

    // converting string size to bytes in little-endian order
    let len_as_bytes: [u8; STR_LEN_BYTES] = mem::transmute((str.len() as u32).to_le()); // todo to fn
    let total_len = STR_LEN_BYTES
        .checked_add(str.len())
        .expect("usize overflow occurred");

    let mut result: Vec<u8> = Vec::with_capacity(total_len);
    result.write_all(&len_as_bytes)?;
    result.write_all(str.as_bytes())?;

    let result_ptr = alloc(total_len)?.as_ptr();
    std::ptr::copy_nonoverlapping(result.as_ptr(), result_ptr, total_len);

    Ok(result_ptr)
}