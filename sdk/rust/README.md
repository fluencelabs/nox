### Introduction

This guide describes how to create a simple application for the Fluence platform using Rust programming language. The Fluence ecosystem is designed to run Webassembly (Wasm) program in decentralized and trustless environments. Generally our ecosystem can be viewed as several logical parts: a `client-side` (a frontend that used for sending requests to Wasm program; developed by user), the `VM wrapper` (an intermediate layer that receives queries from `client side` and routes it to a `Wasm program`) and a `Wasm program` (also developed by user). But an arbitrary Wasm code can't be run on Fluence since it can use some import to host-based functions that environment isn't provided for security reasons or so on. And also each Wasm program has to have some features to be able to run on Fluence, they will be described in this guide in detail.  

### Prerequisites

First of all it needs to install Rust with wasm32-unknown-unknown target. Currently, our SDK requires nightly version of Rust because of using of [Allocator api](https://doc.rust-lang.org/beta/std/alloc/trait.Alloc.html) that currently is experimental. To install Rust you can use the following commands:

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2019-01-08

export $PATH=$PATH:~/.cargo/bin

rustup target add wasm32-unknown-unknown --toolchain nightly-2019-01-08
```

The first command installs Rust compiler and other tools to `~/.cargo/bin`. Note that since nightly Rust api is unstable version of January 8, 2019 is used. The second line is used to update `PATH` environment variable. And the last one installs `wasm32-unknown-unknown` target for Rust to be able to compile code to Wasm.

To check that everything is set up correctly you can use this simple command:

```bash
echo "fn main(){1;}" > test.rs; rustc --target=wasm32-unknown-unknown test.rs
```

It creates file with Rust code and then complies it to Webassembly. If it ends without errors and there is a `test.wasm` file in the same folder, set up is correct.

### Wasm program conventions

Fluence uses so-called `verification game` technique to prove a correctness of computation results. To simplify this process some limitations for Wasm program have been introduced:

1. Wasm program can consist of several Wasm modules with different names but only one of them (lets call it `master`) can be called from user-side. This master module has to don't have the module name section. This requirement is based on the fact that according to the Wasm specification module name is optional and there is no a possibility to add it to resulted Wasm binary by default `rust` compiler.
2. Each `master` module has to have three export (in terms of Wasm specification) functions with names `invoke`, `allocate` and `deallocate`.
3. `invoke` will be used as the main module handler function. It means that all client-side requests is routed to it. The exactly signature of this function has to be `(func (export "invoke") (param $buffer i32) (param $size i32) (result i32))` in wast representation. It receives two i32 params that represent a pointer to supplied argument and its size. If `client-side` send an empty byte buffer `invoke` will be called with two nulls. This function has to return a pointer to result that has to have the next structure in memory: `| size (4 bytes; little endian) | result buffer (size bytes) |`. This convention is based on a fact that Wasm function can return value of i32, i64, f32, f64, simd types.
4. `allocate` function has to have the next signature `(func (export "allocate") (param $size i32) (result i32))` in wast representation. It has to return a pointer as i32 to a module memory region long enough to hold `size` bytes.
5. `deallocate` function has to have the next signature `(func (export "deallocate") (param $address i32) (param $size i32) (return))`. It is called by VM wrapper with pointer to a memory region previously allocated through `allocate` function and its size. This function should free this memory region.

A Wasm module usually is invoked by following scheme:
1. A client-side send a request to Wasm code as a byte array.
2. VM wrapper call `allocate` function of `master` Wasm module with size of the array.
3. VM wrapper writes the array to the module memory.
4. VM wrapper call `invoke` function from `master` module with the address returned from `allocate` function and the array size.
5. VM wrapper synchronously wait of `invoke` result. After receiving a `pointer` from it, reads 4 bytes(that represents `size` of byte array) and then reads `size` bytes from `pointer + 4` offset (`result`).
6. `Result` as a byte array is sent to a client.

### Using Fluence SDK

To a simplify all of these requirements we developed a simple [SDK](https://docs.rs/fluence_sdk) for Rust. It has functions for allocate and deallocate memory regions, to read/write byte array to/from memory and also to print logs from Wasm module.

Lets show all of them on a example of simple Rust program that returns `Hello world!` string. According to the Wasm program conventions a basic structure of Wasm `master` module can be smth like that:

```Rust
#[no_mangle]
pub unsafe fn invoke(_ptr: *mut u8, _len: usize) -> usize {
    ...
}

#[no_mangle]
pub unsafe fn allocate(size: usize) -> NonNull<u8> {
    ...
}

#[no_mangle]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize) {
    ...
}
```
There we have three export functions, two of that (`allocate`/`deallocate`) have only utility purpose and normally used only by VM wrapper. Fluence SDK has `fluence::memory::alloc` and `fluence::memory::dealloc` functions based on GlobalAlloc Rust API that can be used to simplify implementation of these functions. So they can be realized like this:

```Rust
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

``` 

To return string from `invoke` function `fluence::memory::write_str_to_mem` can be used:

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
```

Yeah, now we have a simple program that can be run on Fluence, lets compile it!

### Compilation and test launch

To compile this example fluence_sdk has to be specified in dependencies in Cargo.toml, it minimal version could be like this:

```Toml
[package]
name = "hello world"
version = "0.1.0"

[lib]
crate-type = ["cdylib"]

[dependencies]
fluence_sdk = "0.0.6"
```
Currently the last version of `fluence_sdk` is `0.0.6` but now it under construction and will change in future, stay tuned.

And to compile all of the following command could be used:

```bash
cargo +nightly-2019-01-08 build --target wasm32-unknown-unknown --release
```
    
### Best practices

- don't use panic! and methods that lead to it (expect, unwrap) in your code. 
- avoid using unsafe operations except these that already in Fluence SDK.
