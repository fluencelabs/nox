//! Module for memory management functions

use std::alloc::{Alloc, Global, Layout};
use std::error::Error;
use std::ptr::NonNull;
use std::mem;
use std::io::Write;

pub const STR_LEN_BYTES: usize = 4;

/// Result for all possible Error types.
pub type GenResult<T> = Result<T, Box<Error>>;

/// Allocates memory area of specified size and returns address of the first
/// byte in the allocated memory area.
pub unsafe fn alloc(size: usize) -> GenResult<NonNull<u8>> {
    let layout: Layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Global.alloc(layout).map_err(Into::into)
}

/// Deallocates memory area with first byte address = `ptr` and size = `size`.
pub unsafe fn dealloc(ptr: NonNull<u8>, size: usize) -> GenResult<()> {
    let layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Ok(Global.dealloc(ptr, layout))
}

/// Writes Rust string into the memory directly as string length and byte array
/// (little-endian? order). Written memory structure is:
///     | str_length: 4 BYTES | string_payload: str_length BYTES|
pub unsafe fn put_to_mem(str: String) -> *mut u8 {
    // converting string size to bytes in little-endian order
    let len_as_bytes: [u8; STR_LEN_BYTES] = mem::transmute((str.len() as u32).to_le());
    let mut result: Vec<u8> = Vec::with_capacity(STR_LEN_BYTES + str.len());
    result.write_all(&len_as_bytes).unwrap();
    result.write_all(str.as_bytes()).unwrap();

    let result_ptr = alloc(result.len()).unwrap().as_ptr();

    // writes bytes into memory byte-by-byte. Address of first byte will be == `ptr`
    for (idx, byte) in result.iter().enumerate() {
        std::ptr::write(result_ptr.offset(idx as isize), *byte);
    }

    result_ptr
}