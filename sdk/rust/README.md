### Introduction

This guide describes how to create a simple application for the Fluence platform using Rust programming language. The Fluence ecosystem is designed to run Webassembly (Wasm) program in decentralized and untrusted environment. But it can't run an arbitrary Wasm code since it can uses some import to host-based functions that environment isn't provided for security reasons or so on. And also each Wasm program has to have some features to be able to run on Fluence. They will be described in this guide in detail.  

### Prerequisites

First of all it needs to install Rust with wasm32-unknown-unknown target. Currently, our SDK requires nightly version of Rust because of using of [Allocator api](https://doc.rust-lang.org/beta/std/alloc/trait.Alloc.html) that currently is experimental. To install Rust you can use the following commands:

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2019-01-08

export $PATH=$PATH:~/.cargo/bin

rustup target add wasm32-unknown-unknown --toolchain nightly-2019-01-08
```

The first command installs Rust compiler and other tools to `~/.cargo/bin`. Note that since nightly Rust api is unstable it is used version of January 8, 2019. The second one is used to update `PATH` environment variable. And the last one installs `wasm32-unknown-unknown` target for Rust to be able to compile code to Wasm.

To check that everything is set up correctly you can use this simple command:

```bash
echo "fn main(){1;}" > test.rs; rustc --target=wasm32-unknown-unknown test.rs
```

It creates rust code file and then complies it to Webassembly. If it ends without errors and there is a `test.wasm` file in the same folder, set up is correct.

### Wasm program conventions

Fluence uses so-called `verification game` technique to prove a correctness of computation results. To simplify this process some limitations for Wasm program has been introduced:

1. Wasm program can consist of several Wasm modules with different names but only one of them (`master`) can be called from user-side. This master module has to haven't got the module name section. This requirement is based on the fact that according to the Wasm specification module name is optional and there is no a possibility to add it to resulted Wasm binary by default `rust` compiler.
2. Each `master` module has to have three export (in terms of Wasm specification) functions with names `invoke`, `allocate` and `deallocate`.
3. `invoke` will be used as the main module handler function. It means that all client-side requests is routed to it. The exactly signature of this function has to be `(func (export "invoke") (param $buffer i32) (param $size i32) (result i32))` in wast representation. It receives two i32 params that represent a pointer to supplied argument and its size.  
  This function has to return a pointer to result that has to have the next structure in memory: `| size (4 bytes; little endian) | result buffer (size bytes) |`. This convention is based on a fact that Wasm function can return only value of i32, i64, f32, f64, simd types.
4. `allocate` function has to have the next signature `(func (export "allocate") (param $size i32) (result i32))` in wast representation. It has to return a pointer as i32 to a module memory region long enough to hold `size` bytes.
5. `deallocate` function has to have the next signature `(func (export "deallocate") (param $address i32) (param $size i32) (return))`. It called with argument represents pointer to a memory region previously allocated through `allocate` function and its size. This function should free this memory region.

A Wasm module usually uses by following scheme: 
1. A client-side send a request to Wasm code as a byte array
2. VM wrapper call `allocate` function of `master` Wasm module with size of the array
3. VM wrapper writes the array to module memory
4. VM wrapper call `invoke` function from `master` module with the address returned from `allocate` function and array size
5. VM wrapper synchronously wait of `invoke` results. After receiving a fat `pointer` from it reads 4 bytes from it (`size`) and then reads `size` bytes from `pointer + 4` offset (`result`).
6. Readed `result` as a byte array is send to a client.

### Using Fluence SDK

To a simplify all of these requirements we developed a simple [SDK](https://docs.rs/fluence_sdk) for Rust. It has function for work with memory and to be able to write to output logs from Wasm module.

Lets review it by example. According to the Wasm program conventions a basic structure of Wasm `master` module should be a following:

```Rust
#[no_mangle]
pub unsafe fn invoke(_ptr: *mut u8, _len: usize) -> usize {
}

#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
}

#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
}
```

Fluence SDK provides capabilities to simplify these function:

```Rust
#[no_mangle]
pub unsafe fn invoke(_ptr: *mut u8, _len: usize) -> usize {
    let test_str = "Hello, world!";
    fluence::memory::write_str_to_mem(&test_str)
        .unwrap_or_else(|_| {
            panic!("[Error] Putting the result string into a raw memory was failed")
        })
        .as_ptr() as usize
}

#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Allocation of zero bytes is not allowed."));

    fluence::memory::alloc(non_zero_size)
        .unwrap_or_else(|_| panic!("[Error] Allocation of {} bytes failed.", size))
}

#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    let non_zero_size = NonZeroUsize::new(size)
        .unwrap_or_else(|| panic!("[Error] Deallocation of zero bytes is not allowed."));
    fluence::memory::dealloc(ptr, non_zero_size)
        .unwrap_or_else(|_| panic!("[Error] Deallocate failed for ptr={:?} size={}.", ptr, size));
}
```

### Compilation and test launch
    TODO
    
### Determenism
    Programs running on Fluence VM must be determenistic. To achieve that, the following operations are currently not supported:
    
    Calling external functions (IO, syscalls)
    Floating point operations
    Multithreading and concurrency
    Random number generators
    
    We developed a simple wasm code sanitizer that can check if an external library uses some of unsupported operations automatically.
    
### Best practices
    Don't use panic! and methods that lead to it (expect, unwrap) in your code.
    Avoid using unsafe operations except these that already in Fluence SDK.
