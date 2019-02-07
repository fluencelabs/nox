# Faktorovich zaveschal
1. Business goal
   
   We need this document to make user onboarding possible and easy. Without that document users would have to find a way of using by themselves. That's bad because people rarely would learn something complex without obvious profits.

2. Customer of that document
   
   Alex Demidko and business team are customers for that document. They suffer without it as they can't onboard users or promote Devnet.

3. Target audience
   
   Fellow developers interested in decentralized tech. They don't know how to write Rust or Scala code. They maybe know how to write Javascript code. They maybe know something about decentralized world. They can be brilliant tech geniuses checking out new cool tech or a complete newbie looking for a way of developing her first gambling dApp.

4. What problems target audience have in mind? Why would they read it?
   
   To learn how to use Fluence (description of Fluence is outside of this guide), and what could be solved by Fluence, i.e., what are the profits of Fluence.

5. Reading scenarios
   
   They could follow from top to down, jump around by ToC or scroll everywhere feeding their madness by fast and seemingly uncontrollable screen seziures. It's not a handbook or a reference.

# Off and go!
This guide is aimed for first-time users of Fluence. Following it from top to bottom will leave you with your own decentralized backend that you developed using Rust and Webassembly, and a frontend app that's able to communicate with that backend. It's not gonna be hard!

## The Plan
The plan is as follows.

First, you'll set up Rust and develop a simple backend with it. Then, you will adapt that backend to be ran on Fluence, and compile it to Webassembly.

After that, you'll set up your own Ethereum blockchain and Ethereum Swarm. Then goes bringing a Fluence cluster up to life, and deploying your freshly-developed backend on top of it. You'll learn how to use Fluence CLI to control Fluence network through smart-contract, starting and shutting down Tendermint clusters, uploading WASM code to Swarm and running it in virtual machine. 

Finally, there will be some frontend time. Using `fluence-js` library you will connect to a running Tendermint cluster, and send commands to your backend, and see how all nodes in cluster executing the same code in sync.

## Backend
Now you're Rust Backend Developer. Put up your best nerdy t-shirt, take a deep breath, and go!

### Setting up Rust
Let's get some Rust. 

#### Install rust compiler and it's tools
```bash
# install Rust compiler and other tools to `~/.cargo/bin`
~ $ curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
info: downloading installer
...
    nightly installed
Rust is installed now. Great!
To configure your current shell run source $HOME/.cargo/env
```

Let's listen to the installer and _configure your current shell_:
```bash
~ $ source $HOME/.cargo/env
<no output>
```

Also, to compile Rust to Webassembly, let's add wasm32 compilation target. Just run the following:
```bash
# install target for Webassembly
~ $ rustup target add wasm32-unknown-unknown --toolchain nightly
info: downloading component 'rust-std' for 'wasm32-unknown-unknown'
info: installing component 'rust-std' for 'wasm32-unknown-unknown'
```

To check that everything is set up correctly, let's compile some Rust code:

```bash
# create test.rs with a simple program that returns number 1
~ $ echo "fn main(){1;}" > test.rs
# compile it to wasm
~ $ rustc --target=wasm32-unknown-unknown test.rs
<no output>
# check test.wasm was created
~ $ ls -la test.wasm
-rwxr-xr-x 1 user user 834475 Feb  7 08:12 test.wasm
```

If everything looks similar, then it's time to create a Rust hello-world project!

### Creating a hello-world in Rust
#### Creating an empty Rust package
First, let's create a new empty Rust package:

```bash
# create empty Rust package
~ $ cargo new hello-world --edition 2018
Created binary (application) `hello-world` package

# go to package directory
~ $ cd hello-world
~/hello-world $
```

More info on creating a new Rust project can be found in [Rust docs](https://doc.rust-lang.org/cargo/guide/creating-a-new-project.html).

#### Implementing the hello-world
_If you're familiar with Rust, feel free to skip that section_

Let's code! We want our `hello-world` to receive a username from, well, user, and greet the world on user's behalf.

Open `src/main.rs` in your favourite editor:
```bash
~/hello-world $ edit src/main.rs
```

You will see the following code, it's there by default:
```rust
fn main() {
    println!("Hello, world!");
}
```

It almost does what we need! But we need more, we need to read and print the user name along these lines. So, delete all the code in `main.rs`, and paste the following:
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

What this code does, line-by-line:
1. Brings `std::env` to scope, to use it later
2. Defines a `greeting` function that takes a name, and returns a greeting message
3. Puts `name` inside a greeting message, and returns it
4. Defines a `main` function
5. Reads first program argument to the `name` variable
6. Calls `greeting` with the `name` passed in, and prints the result

Let's now compile and run our example:
```bash
~/hello-world $ cargo run myName
   Compiling hello-world v0.1.0 (/root/hello-world)
    Finished dev [unoptimized + debuginfo] target(s) in 0.70s
     Running `target/debug/hello-world myName`
Hello, world! From user myName
```

_**WARNING:** If you see the following error, you must install `gcc` and try `cargo run` again:_
```bash
   Compiling hello-world v0.1.0 (/root/hello-world)
error: linker cc not found
  |
  = note: No such file or directory (os error 2)

error: aborting due to previous error

error: Could not compile hello-world.
```

Now that we have a working hello-world, it's time to adapt it to be used with Fluence.

### Making it fluency!
#### Adding fluence as a dependency


#### Making library
!!! TODO: explain why lib.rs is needed

You were running your program by executing code in `src/main.rs`, but now we need to convert it to a library. Let's do that by moving `greeting` function to `src/lib.rs`.

Create & open `src/lib.rs` in your editor:
```bash
~/hello-world $ edit src/lib.rs
```

Then paste the following code there:
```rust
use fluence::sdk::*;

#[invocation_handler]
fn greeting(name: String) -> String {
    format!("Hello, world! From user {}", name)
}
```