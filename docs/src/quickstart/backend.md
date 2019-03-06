# Developing the backend app
Fluence runs WebAssembly programs, so it is possible to build a Fluence backend in any language that targets WebAssembly. In this guide we will use Rust as a language of choice. 

First you will build a hello world Rust app, adapt it to Fluence, and then compile it to WebAssembly.

## Setting up Rust
Let's get some Rust!

Install the Rust compiler:
```bash
# installs the Rust compiler and supplementary tools to `~/.cargo/bin`
~ $ curl https://sh.rustup.rs -sSf | sh -s -- -y
info: downloading installer
...
Rust is installed now. Great!
To configure your current shell run source $HOME/.cargo/env
```

Let's listen to the installer and configure your current shell:  
<small>(new shell environments should pick up the right configuration automatically)</small>
```bash
~ $ source $HOME/.cargo/env
<no output>
```

After that, we need to install the nighly Rust toolchain:  
<small>(Fluence Rust SDK requires the nightly toolchain due to certain memory operations)</small>
```bash
~ $ rustup toolchain install nightly
info: syncing channel updates ...
...
  nightly-<arch> installed - rustc 1.34.0-nightly (57d7cfc3c 2019-02-11)
```

Let's check that the nightly toolchain was installed successfully:
```bash
~ $ rustup toolchain list | grep nightly
# the output should contain the nighly toolchain
...
nightly-<arch>
```

To compile Rust to WebAssembly, we also need to add the `wasm32` compilation target:
```bash
# install target for WebAssembly
~ $ rustup target add wasm32-unknown-unknown --toolchain nightly
info: downloading component 'rust-std' for 'wasm32-unknown-unknown'
info: installing component 'rust-std' for 'wasm32-unknown-unknown'
```

Finally, let's check that everything was set up correctly and compile a sample Rust code:

```bash
# create a simple program that always returns 1
~ $ echo "fn main(){1;}" > test.rs

# compile it to WebAssembly using rustc from the nightly toolchain
~ $ rustup run nightly rustc --target=wasm32-unknown-unknown test.rs
<no output>

# check that the test.wasm output file was created
~ $ ls -lh test.wasm
-rwxr-xr-x  1 user  user   1.4M Feb 11 11:59 test.wasm
```

If everything looks similar, then it's time to create a Rust hello-world project!

## Creating an empty Rust package
Let's create a new empty Rust package:

```bash
# create empty Rust package
~ $ cargo +nightly new hello-world --edition 2018
Created binary (application) `hello-world` package

# go to the package directory
~ $ cd hello-world
~/hello-world $
```

More info on creating a new Rust project can be found in the Rust [Cargo book](https://doc.rust-lang.org/cargo/guide/creating-a-new-project.html).

## [Optional] Creating a Hello World Rust application
If you are already familiar with Rust, feel free to skip this section.

Let's write some code: our backend should receive a user name from the program input, and then print a greeting.

Take a look at `src/main.rs`:
```bash
~/hello-world $ cat src/main.rs
```

You will see the following code, which should be there by default and almost does what we need:
```rust
fn main() {
    println!("Hello, world!");
}
```

Open `src/main.rs` in any editor, delete all existing code, and paste the following:
```rust
use std::env;

fn greeting(name: String) -> String {
    format!("Hello, world! -- {}", name)
}

fn main() {
    let name = env::args().nth(1).unwrap();
    println!("{}", greeting(name));
}
```

This code:
1. defines the `greeting` function which takes a name and returns a greeting message
2. defines the `main` function which reads the first argument, passes it to the `greeting` function, and prints the returned result

Let's now compile and run our example:
```bash
~/hello-world $ cargo +nightly run MyName
   Compiling hello-world v0.1.0 (/root/hello-world)
    Finished dev [unoptimized + debuginfo] target(s) in 0.70s
     Running `target/debug/hello-world MyName`
Hello, world! -- MyName
```
***
**WARNING!** If you see the following error, you should install [gcc](https://gcc.gnu.org/install/) and try `cargo +nightly run` again:
```bash
Compiling hello-world v0.1.0 (/root/hello-world)
error: linker cc not found
  |
  = note: No such file or directory (os error 2)

error: aborting due to previous error
error: Could not compile hello-world.
```
***

Now that we have a working Hello World application, it's time to adapt it for Fluence.

## Creating a Hello World backend for Fluence
For a backend to be compatible with the Fluence network, it should follow few conventions to let Fluence nodes run your code correctly. To reduce the amount of boilerplate code, we have developed the Rust SDK. Let's see how to use it.

### Adding Fluence as a dependency
First you need to add the Fluence Rust SDK to as a dependency.  
Let's take a look at `Cargo.toml`:
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

Now, open `Cargo.toml` in the editor, and add `fluence` to `dependencies`:
```toml
[package]
name = "hello-world"
version = "0.1.0"
authors = ["root"]
edition = "2018"

[dependencies]
fluence = {version = "0.0.11"}
```

### Implementing the backend logic
Create and open `~/hello-world/src/lib.rs` in the editor and paste the following code there:
```rust
use fluence::sdk::*;

#[invocation_handler]
fn greeting(name: String) -> String {
    format!("Hello, world! From user {}", name)
}
```

This code imports the Fluence SDK, and marks the `greeting` function with the `#[invocation_handler]` macro. 

The function marked with the `#[invocation_handler]` macro is called a _gateway function_. It is essentially the entry point to your application: all client transactions will be passed to this function, and once it returns a result, clients can read this result. 

Gateway functions are allowed to take and return only `String` or `Vec<u8>` values – check out the [SDK overview](../backend/sdk.md) for more information. 

### Making it a library
For the gateway function to be correctly exported and thus available for Fluence, the backend should be compiled to WebAssembly as a library.

To make the backend a library, open `Cargo.toml` in the editor, and add the `[lib] `section:
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
fluence = { version = "0.0.11"}
```

### Compiling to WebAssembly
To build the `.wasm` file, run this from the application directory:  
<small>(note: downloading and compiling dependencies might take a few minutes)</small>

```bash
~/hello-world $ cargo +nightly build --lib --target wasm32-unknown-unknown --release
    Updating crates.io index
    ...
    Finished release [optimized] target(s) in 1m 16s
```

If everything goes well, you should get the `.wasm` file deep in the target directory.  
Let's check it:
```bash
~/hello-world $ ls -lh target/wasm32-unknown-unknown/release/hello_world.wasm
-rwxr-xr-x  2 user  user  1.4M Feb 11 11:59 target/wasm32-unknown-unknown/release/hello_world.wasm
```
