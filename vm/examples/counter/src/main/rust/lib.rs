#![feature(allocator_api)]
#![feature(alloc)]
extern crate alloc;
extern crate core;
extern crate fluence_sdk as fluence;

use std::ptr::NonNull;

mod counter;

/// Public function for export

static mut COUNTER: counter::Counter = counter::Counter { counter: 0 };

#[no_mangle]
pub unsafe fn inc(_ptr: *mut u8, _len: usize) {
    COUNTER.inc()
}

#[no_mangle]
pub unsafe fn get(_ptr: *mut u8, _len: usize) -> usize {
    fluence::memory::write_str_to_mem(COUNTER.get().to_string())
        .unwrap_or_else(|_| {
            panic!("[Error] Putting the result string into a raw memory was failed")
        })
        .as_ptr() as usize
}

/// Used from the host environment for memory allocation for passed parameters.
#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    fluence::memory::alloc_unsafe(size)
        .unwrap_or_else(|_| panic!("[Error] Allocation of {} bytes failed.", size))
}

/// Used from the host environment for memory deallocation after reading results
/// of function from Wasm memory.
#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    fluence::memory::dealloc_unsafe(ptr, size)
        .unwrap_or_else(|_| panic!("[Error] Deallocate failed for prt={:?} size={}.", ptr, size))
}
