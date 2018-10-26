//! Basic functions for dealing with WASM memory.
//!
//! This module contains functions for initializing and manipulating memory.

pub mod errors;

use self::errors::MemError;
use std::alloc::{Alloc, Global, Layout};
use std::mem;
use std::ptr::NonNull;
use std::num::NonZeroUsize;

/// Result for this module.
pub type MemResult<T> = Result<T, MemError>;

/// Allocates memory area of specified size and returns its address. Actually is
/// just a wrapper for [`Global::alloc`].
///
/// # Safety
///
/// See [`GlobalAlloc::alloc`].
///
pub unsafe fn alloc(size: NonZeroUsize) -> MemResult<NonNull<u8>> {
    let layout: Layout = Layout::from_size_align(size.get(), mem::align_of::<u8>())?;
    Global.alloc(layout).map_err(Into::into)
}

/// Deallocates memory area for current memory pointer and size. Actually is
/// just a wrapper for [`Global::dealloc`].
///
/// # Safety
///
/// See [`GlobalAlloc::alloc`].
///
pub unsafe fn dealloc(ptr: NonNull<u8>, size: NonZeroUsize) -> MemResult<()> {
    let layout = Layout::from_size_align(size.get(), mem::align_of::<u8>())?;
    Ok(Global.dealloc(ptr, layout))
}

/// A number of bytes that code a string length when the string is putting into
/// memory. See [put_to_mem] method.
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
pub unsafe fn write_str_to_mem(str: &str) -> MemResult<NonNull<u8>> {
    // converting string size to bytes in little-endian order
    let len_as_bytes: [u8; STR_LEN_BYTES] = mem::transmute((str.len() as u32).to_le());
    let mut result_vec = len_as_bytes.to_vec();
    result_vec.extend_from_slice(str.as_bytes());
    let result = Ok(NonNull::new_unchecked(result_vec.as_mut_ptr()));
    mem::forget(result_vec);
    result
}

/// Builds Rust string from the pointer and the length. Bytes copying doesn't
/// occur, new String just wraps bytes around. Memory will be deallocated when
/// the String will be dropped.
///
/// # Safety
///
/// The ownership of `ptr` is effectively transferred to the
/// `String` which may then deallocate, reallocate or change the
/// contents of memory pointed to by the pointer at will. Ensure
/// that nothing else uses the pointer after calling this
/// function.
pub unsafe fn deref_str(ptr: *mut u8, len: usize) -> String {
    String::from_raw_parts(ptr, len, len)
}


/// Read Rust String from the raw memory. This operation is opposite of
/// [write_str_to_mem].
unsafe fn read_str_from_fat_ptr(ptr: NonNull<u8>) -> MemResult<String> {
    // read string length from current pointer
    let str_len = read_len(ptr.as_ptr()) as usize;

    // create String for readed string length
    let string_ptr = ptr.as_ptr().add(STR_LEN_BYTES);
    let str = deref_str(string_ptr, str_len);

//    ptr.as_ptr().drop_in_place();
//    dealloc(ptr, NonZeroUsize::new_unchecked(str_len + STR_LEN_BYTES));
    Ok(str)
}



/// Reads u32 from current pointer. Don't affect the specified pointer.
/// You can use the pointer after calling this method as you wish. Don't forget
/// to deallocate memory for this pointer when it's don't need anymore.
unsafe fn read_len(ptr: *mut u8) -> u32 {
    let mut str_len_as_bytes: [u8; STR_LEN_BYTES] = [0; STR_LEN_BYTES];
    std::ptr::copy_nonoverlapping(ptr, str_len_as_bytes.as_mut_ptr(), STR_LEN_BYTES);
    std::mem::transmute(str_len_as_bytes)
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
    fn write_and_read_str_test() { unsafe {
        let src_str = "some string Ω";

        let ptr = write_str_to_mem(src_str.clone()).unwrap();
        let result_str = read_str_from_fat_ptr(ptr).unwrap();
        assert_eq!(src_str, result_str);
    }}


    /// Creates a big string like: "Q..Q" with specified length.
    fn create_big_str(len: usize) -> String {
        unsafe { String::from_utf8_unchecked(vec!['Q' as u8; len]) }
    }

    #[test]
    fn lot_of_write_and_read_str_test() { unsafe {
        let mb_str = create_big_str(1024*1024);

        // writes and read 1mb string (takes several secounds)
        for _ in 1..10_000 {
            let ptr = write_str_to_mem(&mb_str.clone()).unwrap();
            let result_str = read_str_from_fat_ptr(ptr).unwrap();
            assert_eq!(mb_str, result_str);
        }
    }}

}
