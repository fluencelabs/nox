# Backend app conventions

## App lifecycle

The Fluence network backend infrastructure is based on Scala and Rust. Each computation node in the network has the `VM wrapper` written on Scala that interacts with a backend `app` written on Wasm and published to the network by developers. To be able to run supplied Wasm code [Asmble](https://github.com/fluencelabs/asmble) is used. It compiles supplied Wasm code to a JVM class that then loaded by the `VM wrapper`. All other interactions between an `app` and the `VM wrapper` occur in the `VM wrapper` process address space without any inter-process communications.

A `main` module is invoked by the `VM wrapper` according to the following scheme:

1. A `client-side` sends a request to `App` as a byte array.

2. The `VM wrapper` calls `allocate` function of `main` Wasm module with a size of the array.

3. The `VM wrapper` writes the array to memory of the module.

4. The `VM wrapper` calls `invoke` function from `main` module with the address returned from `allocate` function and the array size.

5. The `VM wrapper` synchronously waits of `invoke` result. After receiving a `pointer` from it, reads 4 bytes (that represents `size` of a byte array) and then reads `size` bytes from `pointer + 4` offset (`result`).

6. The `VM wrapper` calls `deallocate` function of `main` Wasm module with the received pointer.

7. Finally, a `result` is sent to a `client-side` as a byte array.

## Fluence App conventions

There are several restriction and conventions that each supplied `app` has to be met:

1. An `app` can consist of several Wasm modules with different names, but only one of them (let's call it `main` module and all other as `side` modules according to the [emcscripten](https://github.com/emscripten-core/emscripten/wiki/Linking#overview-of-dynamic-linking)) can be called from `user-side`. This `main` module MUST don't have the module name section. This requirement is based on the fact that according to the Wasm specification module name is optional, and now there aren't any possibilities to add it to a generated Wasm binary by default `rust` compiler.

2. Each `main` module MUST have three export (in terms of the Wasm specification) functions with names `invoke`, `allocate` and `deallocate`.

3. `invoke` function is used as the `main` module handler function. It means that all client-side requests are routed to it. The exactly signature of this function MUST be `(func (export "invoke") (param $buffer i32) (param $size i32) (result i32))` in wast representation. It receives two i32 params that represent a pointer to supplied argument and its size. If `client-side` send an empty byte buffer `invoke` SHOULD be called with two nulls (it means that according to Fluence protocol implementation honest nodes call `invoke` with nulls but malicious nodes can do anything). This function has to return a pointer to result that MUST have the next structure in memory: `| size (4 bytes; little endian) | result buffer (size bytes) |`. This convention is based on the fact that Wasm function can return only one value of i32, i64, f32, f64, i128 but there both pointer and size should be returned.

4. `allocate` function MUST have the next signature `(func (export "allocate") (param $size i32) (result i32))` in wast representation. It MUST return a pointer as i32 to a module memory region long enough to hold `size` bytes.

5. `deallocate` function MUST have the next signature `(func (export "deallocate") (param $address i32) (param $size i32) (return))`. It is called by the `VM wrapper` with a pointer to a memory region previously allocated by `allocate` function and its size. This function SHOULD free this memory region.

## Writing app without SDK

For a start, let's review how a simple `hello-world` `app` is made without Fluence backend SDK on pure Rust. From [fluence backend conventions](app_conventions.md) it follows that each `app` must have a `main` module with three export functions. Keeping in mind restrictions to their signatures, a basic structure of a `main` module can look like that:

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

Note that `#[no_mangle]` and `pub unsafe` parts of function signature manage function to be exported from a Wasm module (for more information please refer to this [discussion](https://internals.rust-lang.org/t/precise-semantics-of-no-mangle/4098)).
 
The `hello-world` example without SDK from the [quick start](../quickstart/rust.md) can be implemented like this:
 
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

    let result = format!("Hello, world! From user {}", user_name);
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

The full working code of this example can be found [here](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world/app-without-sdk).

## Misc

From the example above it can be seen that `allocate` and `deallocate` functions serve only utility purpose and are normally used only by the `VM wrapper`. These functions aren't any that most of the developers would want to implement in their `app`. Fluence backend SDK provides `fluence::memory::alloc` and `fluence::memory::dealloc` functions also based on [GlobalAlloc](https://doc.rust-lang.org/beta/std/alloc/trait.GlobalAlloc.html). They exported by default after including the sdk and can be disabled by including it with `fluence = {default-features = false}`.
