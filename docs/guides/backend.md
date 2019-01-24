### Introduction

This guide describes how to create a simple application for the Fluence platform using the Rust programming language. The Fluence ecosystem is designed to run Webassembly (Wasm) program decentralized and in an untrusted environment. But it can't run an arbitrary Wasm code since it can uses some import to host-based functions that environment isn't provided for security reaseons or so on. And also each Wasm program has to some features to be able to run on Fluence. In details it will be describes later.

### Prerequisites

At now Fluence supports only Rust as a frontend language for our ecosystem, so it needs to install it first with wasm32-unknown-unknown taget. Currently, our API requires nightly version of Rust because of it uses [Allocator api](https://doc.rust-lang.org/beta/std/alloc/trait.Alloc.html) that is experimental now. To install rust please use the following commands:

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2019-01-08

echo $PATH=$PATH:~/.cargo/bin

rustup target add wasm32-unknown-unknown --toolchain nightly-2019-01-08
```

The first command install Rust complier and other tools to `~/.cargo/bin`. Note that since nightly Rust api is unstable it is used version of January 8, 2019. The second one uses to update `PATH` environment variable to easy Rust launch. And the last one installs `wasm32-unknown-unknown` target for Rust to be able to compile code to Webassembly.

Еo check that everything is set up correctly you can use this simple command:

```bash
echo "fn main(){1;}" > test.rs; rustc --target=wasm32-unknown-unknown test.rs
```

It creates rust code file and then complies it to Webassembly. If it ended without errors and there is a `test.wasm` file in the same folder, set up is correct.

### Wasm program conventions

Fluence used so-called `verification game` technique to prove a computation results correctness. To simplify this process and launches on Fluence exosystem each Wasm programm has some limitations:

1. Wasm program can consist of several Wasm modules with different names but only one of them (`master`) can be called from user-side. This master module has to don't have a module name section. This requirements is based on the fact that according to Wasm specification module name is optional and there is no a possibility to add it by default `rust` compiler.
2. Each master node has to have three export (in terms of Wasm specification) functions which exactly names are `invoke`, `allocate` and `deallocate`.
3. `invoke` will be used as the main module handler function. It means that all user-side requests routes to it. The exactly signature of this function has to be `(func (export "invoke") (param $buffer i32) (param $size i32) (result i32))` in wast representation. It receives two i32 params - the first represents a pointer to supplied agrument and the second one its size. This function returns a so-called fat pointer to result. It has to have the next structure in memory: `| size (4 bytes; little endian) | result buffer (size bytes) |`.
4. `allocate` function has to have the next signature `(func (export "allocate") (param $size i32) (result i32))` in wast representation. It has to return a pointer as i32 to a module memory region long enough to hold `size` bytes.
5. `deallocate` function has to have the next signature `(func (export "deallocate") (param $address i32) (param $size i32) (return))`. It called with argument represents pointer to a memory region previously allocated through `allocate` function and its size. This function should free this memory region.

A Wasm module usually uses by following scheme:
1. A user-side send a request to Wasm code as a byte array
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

Fluence SDK proves capabilities to simplify these function:

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

##### Determenism

Programs running on Fluence VM must be determenistic. To achieve that, the following operations are currently not supported:
1. Calling external functions (IO, syscalls)
2. Floating point operations
3. Multithreading and concurrency
4. Random number generators

We developed a simple [wasm code sanitizer](???) that can check if an external library uses some of unsupported operations automatically.


##### Best practices

1. Don't use `panic!` and methods that lead to it (expect, unwrap) in your code.
2. Avoid using `unsafe` operations except these that already in Fluence SDK.