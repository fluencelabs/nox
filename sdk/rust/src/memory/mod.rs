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

//! Raw API for dealing with Wasm memory.
//!
//! This module contains functions for memory initializing and manipulating.

pub mod errors;

use self::errors::MemError;
use std::alloc::{Alloc, Global, Layout};
use std::mem;
use std::slice;
use std::num::NonZeroUsize;
use std::ptr::{self, NonNull};

/// Result type for this module.
pub type MemResult<T> = ::std::result::Result<T, MemError>;

/// Count of bytes that length of resulted array occupies.
pub const RESULT_SIZE_BYTES: usize = 4;

/// Allocates memory area of specified size and returns its address. Actually is just a wrapper for
/// [`GlobalAlloc::alloc`].
///
/// # Safety
///
/// See [`GlobalAlloc::alloc`].
///
/// [`GlobalAlloc::alloc`]: https://doc.rust-lang.org/core/alloc/trait.GlobalAlloc.html#tymethod.alloc
///
pub unsafe fn alloc(size: NonZeroUsize) -> MemResult<NonNull<u8>> {
    let layout: Layout = Layout::from_size_align(size.get(), mem::align_of::<u8>())?;
    Global.alloc(layout).map_err(Into::into)
}

/// Deallocates memory area for current memory pointer and size. Actually is just a wrapper for
/// [`GlobalAlloc::dealloc`].
///
/// # Safety
///
/// See [`GlobalAlloc::dealloc`].
///
/// [`GlobalAlloc::dealloc`]: https://doc.rust-lang.org/core/alloc/trait.GlobalAlloc.html#tymethod.dealloc
///
pub unsafe fn dealloc(ptr: NonNull<u8>, size: NonZeroUsize) -> MemResult<()> {
    let layout = Layout::from_size_align(size.get(), mem::align_of::<u8>())?;
    Global.dealloc(ptr, layout);
    Ok(())
}

/// Allocates 'RESULT_SIZE_BYTES + result.len()' bytes and writes length of the result as little
/// endianes [RESULT_SIZE_BYTES] bytes and then writes content of 'result'. So the final layout of
/// the result in memory is following:
/// `
///     | array_length: RESULT_SIZE_BYTES bytes (little-endian) | array: $array_length bytes |
/// `
/// This function should normally be used for returning result of `invoke` function. Vm wrapper
/// expects result in this format.
///
pub unsafe fn write_result_to_mem(result: &[u8]) -> MemResult<NonNull<u8>> {
    let result_len = result.len();
    let total_len = result_len
        .checked_add(RESULT_SIZE_BYTES)
        .ok_or_else(|| MemError::new("usize overflow occurred"))?;

    // converts array size to bytes in little-endian
    let len_as_bytes: [u8; RESULT_SIZE_BYTES] = mem::transmute((result_len as u32).to_le());

    // allocates a new memory region for the result
    let result_ptr = alloc(NonZeroUsize::new_unchecked(total_len))?;

    // copies length of array to memory
    ptr::copy_nonoverlapping(
        len_as_bytes.as_ptr(),
        result_ptr.as_ptr(),
        RESULT_SIZE_BYTES,
    );

    // copies array to memory
    ptr::copy_nonoverlapping(
        result.as_ptr(),
        result_ptr.as_ptr().add(RESULT_SIZE_BYTES),
        result_len,
    );

    Ok(result_ptr)
}

/// Builds Rust string from given pointer and length. Bytes copying doesn't
/// occur, new String just wraps bytes around. Memory will be deallocated when
/// the String will be dropped.
///
/// # Safety
///
/// The ownership of `ptr` is effectively transferred to the
/// `String` which may then deallocate, reallocate or change the
/// contents of memory pointed to by the pointer at will. **Ensure
/// that nothing else uses the pointer after calling this
/// function.**
pub unsafe fn deref_str(ptr: *mut u8, len: usize) -> String {
    String::from_raw_parts(ptr, len, len)
}

/// Reads object from memory. This operation is opposite to
/// [write_str_to_mem]. Reads from the raw memory a string length as first
/// [RESULT_SIZE_BYTES] bytes and then reads string for this length. Deallocates
/// first [RESULT_SIZE_BYTES] bytes that corresponded string length and wraps the
/// rest bytes into a Rust string.
///
/// # Safety
///
/// The ownership of `ptr` is effectively transferred to the
/// `String` which may then deallocate, reallocate or change the
/// contents of memory pointed to by the pointer at will. **Ensure
/// that nothing else uses the pointer after calling this
/// function.**
///
pub unsafe fn read_input_from_memory<T>(ptr: NonNull<u8>) -> MemResult<T> {
    // read string length from current pointer
    let input_len = read_len(ptr.as_ptr()) as usize;

    let total_len = RESULT_SIZE_BYTES
        .checked_add(input_len)
        .ok_or_else(|| MemError::new("usize overflow occurred"))?;

    // creates object from raw bytes
    let mut object: T = slice::from_raw_parts(ptr.as_ptr(), total_len);

    // removes size from the beginning of created object, it allows freeing
    // only memory used for keeping object
    {
        object.drain(0..RESULT_SIZE_BYTES);
    }
    // return string without length at the beginning
    Ok(object)
}

/// Reads `u32` in little endian from given pointer. Doesn't affect the specified pointer.
/// You can use the pointer after calling this method as you wish. Don't forget
/// to deallocate memory for this pointer when it's don't need anymore.
pub unsafe fn read_len(ptr: *mut u8) -> u32 {
    let mut len_as_bytes: [u8; RESULT_SIZE_BYTES] = [0; RESULT_SIZE_BYTES];
    ptr::copy_nonoverlapping(ptr, len_as_bytes.as_mut_ptr(), RESULT_SIZE_BYTES);
    mem::transmute(len_as_bytes)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::num::NonZeroUsize;

    #[test]
    fn alloc_dealloc_test() {
        unsafe {
            let size = NonZeroUsize::new_unchecked(123);
            let ptr = alloc(size).unwrap();
            assert_eq!(dealloc(ptr, size).unwrap(), ());
        }
    }

    #[test]
    fn write_and_read_str_test() {
        unsafe {
            let src_str = "some string Ω";

            let ptr = write_result_to_mem(src_str.as_bytes()).unwrap();
            let result_str: String = read_input_from_memory(ptr).unwrap();
            assert_eq!(src_str, result_str);
        }
    }

    /// Creates a big string like: "Q..Q" with specified length.
    fn create_big_str(len: usize) -> String {
        unsafe { String::from_utf8_unchecked(vec!['Q' as u8; len]) }
    }

    #[test]
    fn lot_of_write_and_read_str_test() {
        unsafe {
            let mb_str = create_big_str(1024 * 1024);

            // writes and read 1mb string (takes several seconds)
            for _ in 1..10_000 {
                let ptr = write_result_to_mem(mb_str.as_bytes()).unwrap();
                let result_str: String = read_input_from_memory(ptr).unwrap();
                assert_eq!(mb_str, result_str);
            }
        }
    }

}
