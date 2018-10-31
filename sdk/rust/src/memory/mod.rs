//! Basic functions for dealing with WASM memory.
//!
//! This module contains functions for initializing and manipulating memory.

pub mod errors;

use self::errors::MemError;
use std::alloc::{Alloc, Global, Layout};
use std::mem;
use std::num::NonZeroUsize;
use std::ptr;
use std::ptr::NonNull;

/// Result type for this module.
pub type MemResult<T> = Result<T, MemError>;

/// Allocates memory area of specified size and returns its address. Actually is
/// just a wrapper for [`GlobalAlloc::alloc`].
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

/// When size == 0 works fine for WASM target, but failed for x86. Has to be removed.
#[deprecated(
    since = "0.0.1",
    note = "please use `alloc` instead, will be removed in next version"
)]
pub unsafe fn alloc_unsafe(size: usize) -> MemResult<NonNull<u8>> {
    let layout: Layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Global.alloc(layout).map_err(Into::into)
}

/// Deallocates memory area for current memory pointer and size. Actually is
/// just a wrapper for [`GlobalAlloc::dealloc`].
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

/// When size == 0 works fine for WASM target, but failed for x86. Has to be removed.
#[deprecated(
    since = "0.0.1",
    note = "please use `dealloc` instead, will be removed in next version"
)]
pub unsafe fn dealloc_unsafe(ptr: NonNull<u8>, size: usize) -> MemResult<()> {
    let layout = Layout::from_size_align(size, mem::align_of::<u8>())?;
    Global.dealloc(ptr, layout);
    Ok(())
}

/// A number of bytes that code a string length when the string is putting into
/// memory. See [write_str_to_mem] method.
pub const STR_LEN_BYTES: usize = 4;

/// Writes Rust string to the memory directly as string length and byte array.
/// This method allocates new 'STR_LEN_BYTES + str.len()' bytes and writes the
/// length of the string as first [STR_LEN_BYTES] bytes and then writes copy of
/// 'str' after 'length' as a rest of the bytes.
/// Original 'str' will be destroyed at the end of this method.
///
/// Written memory structure is:
/// `
///     | str_length: $STR_LEN_BYTES BYTES (little-endian) | string_payload: $str_length BYTES |
/// `
pub unsafe fn write_str_to_mem(str: String) -> MemResult<NonNull<u8>> {
    // converting string size to bytes in little-endian order
    let len_as_bytes: [u8; STR_LEN_BYTES] = mem::transmute((str.len() as u32).to_le());
    let mut result_vec = len_as_bytes.to_vec();
    result_vec.extend_from_slice(&str.into_bytes());
    let total_len = NonZeroUsize::new_unchecked(result_vec.len());

    let result_ptr = alloc(total_len)?;
    ptr::copy_nonoverlapping(result_vec.as_ptr(), result_ptr.as_ptr(), total_len.get());
    Ok(result_ptr)
}

/// Builds Rust string from the pointer and the length. Bytes copying doesn't
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

/// Read Rust String from the raw memory. This operation is opposite of
/// [write_str_to_mem]. Reads from the raw memory a string length as first 4
/// bytes and then reads string for this length. Deallocates first 4 bytes that
/// corresponded string length and wraps the rest bytes into a Rust string.
///
/// # Safety
///
/// The ownership of `ptr` is effectively transferred to the
/// `String` which may then deallocate, reallocate or change the
/// contents of memory pointed to by the pointer at will. **Ensure
/// that nothing else uses the pointer after calling this
/// function.**
///
pub unsafe fn read_str_from_fat_ptr(ptr: NonNull<u8>) -> MemResult<String> {
    // read string length from current pointer
    let str_len = read_len(ptr.as_ptr()) as usize;

    // create string for size and string
    let mut str = deref_str(ptr.as_ptr(), str_len + STR_LEN_BYTES);

    // remove size from the begginning of created string, it allow to free
    // only memory used for keeping string length
    {
        str.drain(0..STR_LEN_BYTES);
    }
    // return string without length at the beginning
    Ok(str)
}

/// Reads `u32` from current pointer. Don't affect the specified pointer.
/// You can use the pointer after calling this method as you wish. Don't forget
/// to deallocate memory for this pointer when it's don't need anymore.
unsafe fn read_len(ptr: *mut u8) -> u32 {
    let mut str_len_as_bytes: [u8; STR_LEN_BYTES] = [0; STR_LEN_BYTES];
    ptr::copy_nonoverlapping(ptr, str_len_as_bytes.as_mut_ptr(), STR_LEN_BYTES);
    mem::transmute(str_len_as_bytes)
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

            let ptr = write_str_to_mem(src_str.to_string()).unwrap();
            let result_str = read_str_from_fat_ptr(ptr).unwrap();
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

            // writes and read 1mb string (takes several secounds)
            for _ in 1..5_000 {
                let ptr = write_str_to_mem(mb_str.to_string()).unwrap();
                let result_str = read_str_from_fat_ptr(ptr).unwrap();
                assert_eq!(mb_str, result_str);
            }
        }
    }

}
