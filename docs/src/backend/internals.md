# Backend internals

## Application interface

A backend application is allowed to consist of more than one WebAssembly module (provided they have different names), but only one of those modules can be called by the state machine. Note that this module **should not** have the module name section. Additionally, every backend application that is going to be deployed to the Fluence network is **expected to** export three functions – `invoke()`, `allocate()`, and `deallocate()`:

```Rust
#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> usize {
    ...
}

#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    ...
}

#[no_mangle]
pub unsafe fn deallocate(ptr: *mut u8, size: usize) {
    ...
}
```

Note that `#[no_mangle]` and `pub unsafe` signature parts force these functions to be exported by WebAssembly (refer to this [discussion](https://internals.rust-lang.org/t/precise-semantics-of-no-mangle/4098) for more info). In _wast_ format, these functions are expected to have the following signatures:

```
(func (export "invoke") (param $buffer i32) (param $size i32) (result i32))

(func (export "allocate") (param $size i32) (result i32))

(func (export "deallocate") (param $address i32) (param $size i32) (return))
```

The `invoke()` function is the main entry point to the deployed application. It takes two `i32` params – a pointer to the byte array stored in the WebAssembly memory and the byte array size. If the state machine needs to call the `invoke()` function with no arguments, it passes two null values for the pointer and the size argument. To return the result, the `invoke()` function returns a pointer to the WebAssembly memory region where the first 4 bytes represent the size of the result buffer, and the next `size` bytes represent the result buffer itself.

The `allocate()` function is responsible for allocating a region of memory where the state machine can write the data which should be passed to the application. Once the region is allocated, the application should not overwrite it until a corresponding `deallocate()` call is made. The `allocate()` function takes an `i32` parameter which specifies the size of the region, and returns a pointer to the allocated region in the WebAssembly memory.

The `deallocate()` function is responsible for memory deallocation. It takes two `i32` arguments – the address of the memory region that should be deallocated and its size.

## Request-response lifecycle

Once the state machine receives a transaction block, it forwards each transaction to the backend application and awaits results of its execution. The transaction processing lifecycle can be described as follows:

1. The state machine calls the `allocate()` function exported by the application and passes to it the size of the memory region that the backend application should allocate for the input. The  `allocate()` function returns the offset in the WebAssembly memory where the input should be written to.

1. The state machine writes the input at the offset returned by the `allocate()` function.

1. The state machine calls the `invoke()` function exported by the application and passes to it the offset where the input was written to and the size of the input in bytes.

1. The state machine synchronously waits for the `invoke()` function to complete. The `invoke()` function returns a pointer to the WebAssembly memory region storing the returned result. The state machine reads the returned result and caches it before sending back to the client.

1. The state machine uses the `deallocate()` function exported by the application to free WebAssembly memory regions used to store input and output data.

## Unmanaged application example

Here is how a simple hello world application can be implemented without the Fluence SDK:

 
```Rust
#![feature(allocator_api)]

use std::alloc::{Alloc, Global, Layout};
use std::mem;
use std::num::NonZeroUsize;
use std::ptr::{self, NonNull};

#[no_mangle]
pub unsafe fn invoke(ptr: *mut u8, len: usize) -> NonNull<u8> {
    let raw_string = Vec::from_raw_parts(ptr, len, len);
    let user_name = String::from_utf8(raw_string).unwrap();

    let result = format!("Hello, world! -- {}", user_name);
    const RESULT_SIZE_BYTES: usize = 4;

    let result_len = result.len();
    let total_len = result_len
        .checked_add(RESULT_SIZE_BYTES)
        .expect("usize overflow occurred");

    // converts array size to bytes in little-endian
    let len_as_bytes: [u8; RESULT_SIZE_BYTES] = mem::transmute((result_len as u32).to_le());

    // allocates a new memory region for the result
    let result_ptr = allocate(total_len);

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

    result_ptr
}

#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    let non_zero_size =
        NonZeroUsize::new(size).expect("[Error]: allocation of zero bytes is not allowed.");
    let layout: Layout = Layout::from_size_align(non_zero_size.get(), mem::align_of::<u8>())
        .unwrap_or_else(|_| panic!("[Error]: layout creation failed while allocation"));
    Global
        .alloc(layout)
        .unwrap_or_else(|_| panic!("[Error]: allocation of {} bytes failed", size))
}

#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    let non_zero_size =
        NonZeroUsize::new(size).expect("[Error]: deallocation of zero bytes is not allowed.");
    let layout = Layout::from_size_align(non_zero_size.get(), mem::align_of::<u8>())
        .unwrap_or_else(|_| panic!("[Error]: layout creation failed while deallocation"));;
    Global.dealloc(ptr, layout);
}
```
