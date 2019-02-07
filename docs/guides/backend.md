### Introduction

This guide goes through the basics of application creation for the Fluence Network using simple Rust application as an example. 

The Fluence ecosystem is designed to run Webassembly (Wasm) program in decentralized trustless environments. Generally it can be considered as several logical parts:
- a `client-side` (a frontend used for sending requests to Wasm program; developed by user),
- the `VM wrapper` (an intermediate layer that receives queries from `client side` and routes it to a `Wasm program`) and 
- a `Wasm program` (also developed by user).

But an arbitrary Wasm code can't be run on Fluence - for example, it can use some imports of host-based functions that environment isn't provided for security reasons. And also each Wasm program has to follow some conventions to be able to interact with `VM wrapper`. These are described in details in `Wasm program conventions` section of this guide.  

### Prerequisites

First of all, it needs to install Rust with Webassembly target. Currently, our SDK requires a nightly version of Rust since it uses some features that presently experimental. To install Rust, you can use the following commands:

```bash
# installs Rust compiler and other tools to `~/.cargo/bin`
$ curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly-2019-01-08

# updates `PATH` environment variable
$ source $HOME/.cargo/env

# installs target for Webassembly
$ rustup target add wasm32-unknown-unknown --toolchain nightly-2019-01-08
```

To check that everything is set up correctly you can use this simple command:

```bash
$ echo "fn main(){1;}" > test.rs; rustc --target=wasm32-unknown-unknown test.rs

$ ls -la test.wasm
-rwxr-xr-x  1 trofim  staff   1.9M Feb  7 02:09 test.wasm*

```

It creates file with Rust code and then complies it to Webassembly. If it ends without errors and there is a `test.wasm` file in the same folder, set up is correct.

### Quick start

At first lets create a new empty Rust lib package f.e. by the following command:

```bash
$ cargo new hello-user --lib --edition 2018
Created library `hello-user` package

$ cd hello-user
```

These commands create a stub for our project (more detailed info about package creating can be found [here](https://doc.rust-lang.org/cargo/guide/creating-a-new-project.html)) and change current directory to it.

Let's show how to write a simple application by an example of a Rust program that receives a `user name` and returns "Hello from Fluence to `user name`". 

We'll write out code in src/lib.rs:

```Rust
use fluence::sdk::*;

#[invocation_handler]
fn main(name: String) -> String {
    format!("Hello from Fluence to {}", name)
}
```

And that's it! Pretty simple, isn't it?

Yeah, now we have a simple program that can be run on Fluence, lets compile it!

### Compilation and test launch

`fluence` crate has to be specified in dependencies in `Cargo.toml` to compile this example. A minimal example `Cargo.toml` could look like this:

```Toml
[package]
name = "hello_user"
version = "0.1.1"
edition = "2018"


[lib]
name = "hello_user"
path = "src/lib.rs"
crate-type = ["cdylib"]

[dependencies]
fluence = { version = "0.0.8", features = ["export_allocator"]}
```

Currently, the last version of `fluence_sdk` is `0.0.8` but now it under construction and will change in future, stay tuned.

And finally to compile this example the following command could be used:

```bash
$ cargo +nightly-2019-01-08 build --target wasm32-unknown-unknown --release
   Compiling proc-macro2 v0.4.27
   ...
    Finished release [optimized] target(s) in 21.97s
```

The worked example of `hello-user` program can be found [here](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-user).

### Wasm program publishing

To publish resulted program `fluence/cli` can be used. This utility has a rich help that can be viewed by
```bash
$ fluence help publish
...
USAGE:
    fluence publish [FLAGS] [OPTIONS] <path> <contract_address> <account>
    ...
```

By example for some setup the following command:

```bash
$ fluence publish \
            --code_path        fluence/vm/examples/hello-user/target/wasm32-unknown-unknown/release/hello-user.wasm \
            --contract_address 0x9995882876ae612bfd829498ccd73dd962ec950a \
            --account          0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --cluster_size     4 \
            --secret_key       0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133
```

publishes `hello-user.wasm` to contract identified by `0x9995882876ae612bfd829498ccd73dd962ec950a`, account `0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d` to cluster of four nodes authenticated by secret key `0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133`. To find more information about cli interface please look at [miner guide](https://github.com/fluencelabs/fluence/blob/master/docs/guides/miner.md).

### Wasm program conventions

Fluence uses so-called `verification game` technique to prove the correctness of computation results. Since that some limitations for Wasm program have been introduced:

1. Wasm program can consist of several Wasm modules with different names, but only one of them (let's call it `principal`) can be called from user-side. This master module HAS TO don't have the module name section. This requirement is based on the fact that according to the Wasm specification module name is optional, and there is no a possibility to add it to a generated Wasm binary by default `rust` compiler.
2. Each `principal` module HAS TO have three export (regarding Wasm specification) functions with names `invoke`, `allocate` and `deallocate`.
3. `invoke` will be used as the main module handler function. It means that all client-side requests are routed to it. The exactly signature of this function HAS TO be `(func (export "invoke") (param $buffer i32) (param $size i32) (result i32))` in wast representation. It receives two i32 params that represent a pointer to supplied argument and its size. If `client-side` send an empty byte buffer `invoke` SHOULD be called with two nulls. This function has to return a pointer to result that has to have the next structure in memory: `| size (4 bytes; little endian) | result buffer (size bytes) |`. This convention is based on the fact that Wasm function can return value of i32, i64, f32, f64, i128 types.
4. `allocate` function HAS TO have the next signature `(func (export "allocate") (param $size i32) (result i32))` in wast representation. It HAS TO return a pointer as i32 to a module memory region long enough to hold `size` bytes.
5. `deallocate` function HAS TO have the next signature `(func (export "deallocate") (param $address i32) (param $size i32) (return))`. It is called by VM wrapper with a pointer to a memory region previously allocated through `allocate` function and its size. This function SHOULD free this memory region.

A Wasm module usually is invoked by the following scheme:
1. A `client-side` send a request to Wasm code as a byte array.
2. `VM wrapper` call `allocate` function of `master` Wasm module with a size of the array.
3. `VM wrapper` writes the array to the module memory.
4. `VM wrapper` call `invoke` function from `master` module with the address returned from `allocate` function and the array size.
5. `VM wrapper` synchronously waits of `invoke` result. After receiving a `pointer` from it, reads 4 bytes(that represents `size` of a byte array) and then reads `size` bytes from `pointer + 4` offset (`result`).
6. `Result` as a byte array is sent to a client.

### Best practices

- don't use panic! and methods that lead to it (expect, unwrap) in your code.
 
- avoid using unsafe operations except these that already in Fluence SDK.

- please pay attention to module memory usage - in many situations module can have public API that explicitly or indirectly consume module memory. So user can make a DoS attack to such programs.
