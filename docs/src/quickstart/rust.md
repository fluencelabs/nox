# Developing the backend app
Fluence runs Webassembly programs, so it's possible to build a Fluence backend in any program language that targets Wasm. In this guide, we will use Rust as a language of choice. 

First you will build a simple hello-world backend in Rust, adapt it to be used with Fluence, and compile to Webassembly.

## Setting up Rust
Let's get some Rust. 

Install rust compiler and it's tools:
```bash
# install Rust compiler and other tools to `~/.cargo/bin`
~ $ curl https://sh.rustup.rs -sSf | sh -s -- -y
info: downloading installer
...
Rust is installed now. Great!
To configure your current shell run source $HOME/.cargo/env
```

Let's listen to the installer and configure your current shell:
```bash
~ $ source $HOME/.cargo/env
<no output>
```

Fluence Rust SDK [uses custom allocator (TODO: link)](???) for copying bytes to and from virtual machine internal memory, and that requires nightly toolchain. To install nightly toolchain, run:
```bash
~ $ rustup toolchain install nightly
info: syncing channel updates ...
...
  nightly-<arch> installed - rustc 1.34.0-nightly (57d7cfc3c 2019-02-11)
```

To check nightly toolchain was installed succesfully:
```bash
~ $ rustup toolchain list | grep nightly
# output should contain nighly toolchain
...
nightly-<arch>
```

Also, to be able to compile Rust to WebAssembly, we need to add wasm32 compilation target. Just run the following:
```bash
# install target for WebAssembly
~ $ rustup target add wasm32-unknown-unknown --toolchain nightly
info: downloading component 'rust-std' for 'wasm32-unknown-unknown'
info: installing component 'rust-std' for 'wasm32-unknown-unknown'
```

To check that everything is set up correctly, let's compile some Rust code:

```bash
# create test.rs with a simple program that returns number 1
~ $ echo "fn main(){1;}" > test.rs

# compile it to wasm using rustc from nightly toolchain
~ $ rustup run nightly rustc --target=wasm32-unknown-unknown test.rs
<no output>

# check test.wasm was created
~ $ ls -lh test.wasm
-rwxr-xr-x  1 user  user   1.4M Feb 11 11:59 test.wasm
```

If everything looks similar, then it's time to create a Rust hello-world project!

## Creating an empty Rust package
First, let's create a new empty Rust package:

```bash
# create empty Rust package
~ $ cargo +nightly new hello-world --edition 2018
Created binary (application) `hello-world` package

# go to the package directory
~ $ cd hello-world
~/hello-world $
```

More info on creating a new Rust project can be found in [Rust docs](https://doc.rust-lang.org/cargo/guide/creating-a-new-project.html).

## Optional: Creating a Hello World Rust application
If you are familiar with Rust, feel free to [skip](#creating-a-fluence-hello-world-backend) that section.

Let's write some code. Our backend should be able to receive a username from program arguments, and print greeting with the username in it.

Take a look at `src/main.rs`:
```bash
~/hello-world $ cat src/main.rs
```

You will see the following code. It's there by default:
```rust
fn main() {
    println!("Hello, world!");
}
```

It almost does what we need, except for reading a username. So, open `src/main.rs` in your editor, delete all the code in there, and paste the following:
```rust
use std::env;

fn greeting(name: String) -> String {
    format!("Hello, world! From user {}", name)
}

fn main() {
    let name = env::args().nth(1).unwrap();
    println!("{}", greeting(name));
}
```

What this code does:
1. Defines a `greeting` function that takes a name, and returns a greeting message
2. Defines a `main` function that reads first program argument, passes it to `greeting`, and prints the result

Let's now compile and run our example:
```bash
~/hello-world $ cargo +nightly run myName
   Compiling hello-world v0.1.0 (/root/hello-world)
    Finished dev [unoptimized + debuginfo] target(s) in 0.70s
     Running `target/debug/hello-world myName`
Hello, world! From user myName
```
***
**WARNING:** If you see the following error, you should install `gcc` and try `cargo +nightly run` again:
```bash
Compiling hello-world v0.1.0 (/root/hello-world)
error: linker cc not found
  |
  = note: No such file or directory (os error 2)

error: aborting due to previous error
error: Could not compile hello-world.
```
***

Now that we have a working hello world, it's time to adapt it to be used with Fluence.

## Creating a Fluence Hello World backend
For a backend to be compatible with Fluence network, it should [follow a few conventions](backend.md#wasm-program-conventions), so Fluence knows how to call your code correctly. To reduce boilerplate and make it easier, we developed a Fluence Rust SDK. Let's see how to use it.

### Adding Fluence as a dependency
First you need to add it to `Cargo.toml` as a dependency. Let's take a look at `Cargo.toml`:
```bash
~/hello-world $ cat Cargo.toml
```

It should look like this:
```toml
[package]
name = "hello-world"
version = "0.1.0"
authors = ["root"]
edition = "2018"

[dependencies]
```

Now, open `Cargo.toml` in your editor, and add `fluence` to `dependencies`:
```toml
[package]
name = "hello-world"
version = "0.1.0"
authors = ["root"]
edition = "2018"

[dependencies]
fluence = { version = "0.0.8", features = ["export_allocator"]}
```

### Implementing backend greeting logic
Create & open `~/hello-world/src/lib.rs` in your editor and paste the following code there:
```rust
use fluence::sdk::*;

#[invocation_handler]
fn greeting(name: String) -> String {
    format!("Hello, world! From user {}", name)
}
```

This code imports Fluence SDK, and marks `greeting` function with `#[invocation_handler]`, so Fluence knows how to call it.

Function marked with `#[invocation_handler]` is called a _gateway function_. It is an entrypoint to your application, all transactions sent by users will be passed to that function, and it's result will be available to users. Gateway function can receive and return either `String` or `Vec<u8>`. 

### Making it a library
For a gateway function to be exported and available for Fluence to call, backend should be compiled to WebAssembly as a library.

To make your backend a library, open `Cargo.toml` in your editor, and paste the following there:
```toml
[package]
name = "hello-world"
version = "0.1.0"
authors = ["root"]
edition = "2018"

[lib]
name = "hello_world"
path = "src/lib.rs"
crate-type = ["cdylib"]

[dependencies]
fluence = { version = "0.0.8", features = ["export_allocator"]}
```

### Compiling to WebAssembly
Run the following code to build a `.wasm` file from your Rust code.

NOTE: Downloading and compiling dependencies might take a few minutes.

```bash
~/hello-world $ cargo +nightly build --lib --target wasm32-unknown-unknown --release
    Updating crates.io index
    ...
    Finished release [optimized] target(s) in 1m 16s
```

If everything goes well, you should have a `.wasm` file deep in `target`. Let's check it:
```bash
~/hello-world $ ls -lh target/wasm32-unknown-unknown/release/hello_world.wasm
-rwxr-xr-x  2 user  user  1.4M Feb 11 11:59 target/wasm32-unknown-unknown/release/hello_world.wasm
```
