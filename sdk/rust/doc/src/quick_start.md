# Quick start

This part goes through the basics of application creation for the Fluence Network using simple Rust application as an example.

### Prerequisites

First of all, you need to install Rust with Webassembly target. Currently, our SDK requires a nightly version of Rust since it uses some features that presently experimental (e.g., [allocator api](https://docs.rs/allocator_api)). To install Rust, you can use the following commands:

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
-rwxr-xr-x  1 user  user   1.9M Feb  7 02:09 test.wasm*

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
fluence = { version = "0.0.11", features = ["export_allocator"] }
```

Currently, the last version of `fluence_sdk` is `0.0.11` but now it under construction and will change in future, stay tuned.

And finally to compile this example the following command could be used:

```bash
$ cargo +nightly-2019-01-08 build --target wasm32-unknown-unknown --release
   Compiling proc-macro2 v0.4.27
   ...
    Finished release [optimized] target(s) in 21.97s
```

The worked example of `hello-user` program can be found [here](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world).
