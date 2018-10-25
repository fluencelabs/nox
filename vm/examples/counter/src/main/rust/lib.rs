#![feature(allocator_api)]
#![feature(extern_prelude)]

#![feature(alloc)]
extern crate alloc;
extern crate core;

use std::ptr::NonNull;

mod counter;
mod memory_manager;

use memory_manager::{alloc, dealloc, put_to_mem};

/// Public function for export

static mut COUNTER_: counter::Counter = counter::Counter { counter: 0 };

#[no_mangle]
pub unsafe fn inc(_ptr: *mut u8, _len: usize) {
    COUNTER_.inc()
}

#[no_mangle]
pub unsafe fn get(_ptr: *mut u8, _len: usize) -> usize {
    put_to_mem(COUNTER_.get().to_string()) as usize
}


/// Used from the host environment for memory allocation for passed parameters.
#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    alloc(size)
        .expect(format!("[Error] Allocation of {} bytes failed.", size).as_str())
}

/// Used from the host environment for memory deallocation after reading results
/// of function from Wasm memory.
#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) -> () {
    dealloc(ptr, size)
        .expect(format!("[Error] Deallocate failed for prt={:?} size={}.", ptr, size).as_str())
}
