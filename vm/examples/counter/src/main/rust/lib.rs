//! Wrapper for counter (a test for Fluence network).
//!
//! This example has capabilities only for incrementing counter and getting its current state.

#![feature(allocator_api)]
#![feature(alloc)]
extern crate alloc;
extern crate core;
extern crate fluence_sdk as fluence;
use std::num::NonZeroUsize;

use std::ptr::NonNull;

mod counter;

//
// FFI for interact with counter module
//

static mut COUNTER: counter::Counter = counter::Counter { counter: 0 };

#[no_mangle]
pub unsafe fn inc(_ptr: *mut u8, _len: usize) {
    COUNTER.inc()
}

#[no_mangle]
pub unsafe fn get(_ptr: *mut u8, _len: usize) -> usize {
    fluence::memory::write_str_to_mem(&COUNTER.get().to_string())
        .unwrap_or_else(|_| {
            panic!("[Error] Putting the result string into a raw memory was failed")
        })
        .as_ptr() as usize
}

/// Allocates memory area of specified size and returns its address.
/// Used from the host environment for memory allocation while parameters passing.
#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Allocation of zero bytes is not allowed."));
    fluence::memory::alloc(non_zero_size)
        .unwrap_or_else(|_| panic!("[Error] Allocation of {} bytes failed.", size))
}

/// Deallocates memory area for provided memory pointer and size.
/// Used from the host environment for memory deallocation after reading results
/// of function from Wasm memory.
#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Deallocation of zero bytes is not allowed."));
    fluence::memory::dealloc(ptr, non_zero_size)
        .unwrap_or_else(|_| panic!("[Error] Deallocate failed for ptr={:?} size={}.", ptr, size));
}
