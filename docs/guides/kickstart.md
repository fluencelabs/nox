- [Off and go!](#off-and-go)
  - [The Plan](#the-plan)
  - [Developing the backend app](#developing-the-backend-app)
    - [Setting up Rust](#setting-up-rust)
      - [Install rust compiler and it's tools](#install-rust-compiler-and-its-tools)
    - [Creating a hello-world in Rust](#creating-a-hello-world-in-rust)
      - [Creating an empty Rust package](#creating-an-empty-rust-package)
      - [Implementing the hello-world](#implementing-the-hello-world)
    - [Making it fluency!](#making-it-fluency)
      - [Adding fluence as a dependency](#adding-fluence-as-a-dependency)
      - [Making it a library](#making-it-a-library)
      - [Making it a cdylib](#making-it-a-cdylib)
      - [Compiling to Webassembly](#compiling-to-webassembly)
  - [Publishing your app](#publishing-your-app)
    - [Connect to Swarm and Ethereum Kovan](#connect-to-swarm-and-ethereum-kovan)
    - [TODO: Registering an Ethereum Kovan account](#todo-registering-an-ethereum-kovan-account)
    - [Installing Fluence CLI](#installing-fluence-cli)
    - [Publishing via Fluence CLI](#publishing-via-fluence-cli)
    - [Check app status](#check-app-status)
  - [Frontend](#frontend)
    - [Preparing web app](#preparing-web-app)
    - [Running and using](#running-and-using)

# Off and go!
This guide aims first-time users of Fluence. Following it from top to bottom will leave you with your own decentralized backend that you developed using Rust and Webassembly, and a frontend app that's able to communicate with that backend. It's not going to be hard!

## The Plan
The plan is as follows.

First, you'll set up Rust and develop a simple backend with it. Then, you will change a few lines so backend can be deployed on Fluence, and compile it to Webassembly.

Then, you'll upload WASM code to Swarm, and publish it to Fluence Devnet smart-contract. For that, you will need a connection to any Ethereum node on Kovan testnet. Publishing your app will bring a new Fluence cluster up to life with your backend on top. You will learn how to check it's status or shut it down. 

Finally, there will be some cozy frontend time. Using `fluence` Javascript library, you will connect to a running Tendermint cluster, and send commands to your backend, and see how all nodes in the cluster are executing the same code in sync.

## Developing the backend app
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

# go to the package directory
~ $ cd hello-world
~/hello-world $
```

More info on creating a new Rust project can be found in [Rust docs](https://doc.rust-lang.org/cargo/guide/creating-a-new-project.html).

#### Implementing the hello-world
_If you're familiar with Rust, feel free to skip that section_

Let's code! We want our `hello-world` to receive a username from, well, user, and greet the world on user's behalf.

Open `src/main.rs` in your favorite editor:
```bash
~/hello-world $ edit src/main.rs
```

You will see the following code. It's there by default:
```rust
fn main() {
    println!("Hello, world!");
}
```

It almost does what we need! But we need more. We need to read and print the user name along these lines. So, delete all the code in `main.rs`, and paste the following:
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
To use the fluence Rust SDK, you need to add it to `Cargo.toml` as a dependency.

Open `Cargo.toml` in your editor:
```
~/hello-world $ edit Cargo.toml
```

You should see the following
```toml
[package]
name = "hello-world"
version = "0.1.0"
authors = ["root"]
edition = "2018"

[dependencies]
```

Now, add fluence to `dependencies`:
```toml
[package]
name = "hello-world"
version = "0.1.0"
authors = ["root"]
edition = "2018"

[dependencies]
fluence = { version = "0.0.8", features = ["export_allocator"]}
```

#### Making it a library
!!! TODO: explain why it should be a library

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

What this code does, line-by-line:
1. Imports fluence SDK
2. Marks our `greeting` function as `#[invocation_handler]`, so Fluence SDK knows it should call it
3. Defines `greeting` function, the same as it was before
4. Creates and returns a greeting message, the same as it was before

#### Making it a cdylib
Uh oh, scary words! Don't be scared, though. It's just another copy-paste exercise. 

Open `Cargo.toml`:
```bash
~/hello-world $ edit Cargo.toml
```

And paste the following there:
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

The only thing changed is the new `[lib]` section. Let's not worry about that now, and try compiling the project instead.

#### Compiling to Webassembly
Run the following code to build a `.wasm` file from your Rust code.
_NOTE: Downloading and compiling dependencies might take a few minutes._

```bash
~/hello-world $ cargo +nightly build --lib --target wasm32-unknown-unknown --release
    Updating crates.io index
    ...
    Finished release [optimized] target(s) in 1m 16s
```

If everything goes well, you should have a `.wasm` file deep in `target`. Let's check it:
```bash
~/hello-world $ stat target/wasm32-unknown-unknown/release/hello_world.wasm
  File: target/wasm32-unknown-unknown/release/hello_world.wasm
  Size: 838385        Blocks: 1640       IO Block: 4096   regular file
  ...
```

## Publishing your app
### Connect to Swarm and Ethereum Kovan
To publish a backend app to Fluence network, you need to upload it to Swarm, and then send its location in Swarm to a Fluence smart contract on Ethereum Kovan testnet. 

Now that's a lot of tech-name-throwing! Let me explain a bit.

- Swarm is a decentralized file storage, so it's like the Dropbox, but more nerdy. 
- Ethereum Kovan testnet is one of the many Ethereum networks, but there's no real money in there, so it's safe and handy for trying out something new.
- Fluence smart contract is what rules the Fluence network and allows users to use it.

So, to upload anything to Swarm, you need to have access to one of its nodes. The same with Ethereum, you will need a connection to any Ethereum node on Kovan testnet.

**We will use existing Ethereum & Swarm nodes, but if you wish, you can [use your own nodes](miner.md) or any other.**

### TODO: Registering an Ethereum Kovan account
TODO

### Installing Fluence CLI
You can download Fluence CLI from here https://github.com/fluencelabs/fluence/releases/tag/cli-0.1.2

Or in terminal:
**Linux**
```bash
~ $ wget https://github.com/fluencelabs/fluence/releases/download/cli-0.1.2/fluence-cli-0.1.2-linux-x64 -O fluence
```

**macOS**
```bash
~ $ curl -L https://github.com/fluencelabs/fluence/releases/download/cli-0.1.2/fluence-cli-0.1.2-mac-x64 -o fluence
```

And finally don't forget to add executable permission:
```bash
~ $ chmod +x ./fluence
~ $ ./fluence --version
Fluence CLI 0.1.2
```

If you see cli's version, proceed to the next step.

### Publishing via Fluence CLI
First, you will need some info prepared:
- Ethereum Kovan account with some money on it
  - You can [get money from faucet](https://github.com/kovan-testnet/faucet)
- A private key for your Ethereum Kovan account. It could be either in hex or as a keystore JSON file.
  - For the keystore file you will also need a password. For more info on keystore file, please read [cli README](../cli/README.md#keystore-json-file).


Now you're ready to publish your app. _Examples below will specify a cluster size of 4 nodes for your app. Adjust it to your needs._

If you have your private key **in hex**, run the following in your terminal, replacing `<>` by actual values:
```bash
~ $ ./fluence publish \
            --eth_url          http://207.154.240.52:8545 \
            --swarm_url        http://207.154.240.52:8500 \
            --code_path        ~/hello-world/target/wasm32-unknown-unknown/release/hello_world.wasm \
            --contract_address 0xe9bbe60d525c7c5d4f3d85036f3ea23003879106 \
            --account          <your ethereum address> \
            --secret_key       <your ethereum private key> \
            --cluster_size     4 \
            --wait_syncing \
            --wait
```

If you have a JSON **keystore file**, run the following in your terminal, replacing `<>` by actual values:

```bash
~ $ ./fluence publish \
            --eth_url          http://207.154.240.52:8545 \
            --swarm_url        http://207.154.240.52:8500 \
            --code_path        ~/hello-world/target/wasm32-unknown-unknown/release/hello_world.wasm \
            --contract_address 0xe9bbe60d525c7c5d4f3d85036f3ea23003879106 \
            --account          <your ethereum address> \
            --keystore         <path to keystore> \
            --password         <password for keystore> \
            --cluster_size     4 \
            --wait_syncing \
            --wait
```
_There is more info on using keystore files with Fluence CLI in it's [README](../cli/README.md#keystore-json-file)._


After running the command, you will see an output similar to the following:
```bash
[1/3]   Application code uploaded. ---> [00:00:00]
swarm hash: 0xf5c604478031e9a658551220da3af1f086965b257e7375bbb005e0458c805874
[2/3]   Transaction publishing app was sent. ---> [00:00:03]
  tx hash: 0x5552ee8f136bce0b020950676d84af00e4016490b8ee8b1c51780546ad6016b7
[3/3]   Transaction was included. ---> [00:02:38]
App deployed.
  app id: 2
  tx hash: 0x5552ee8f136bce0b020950676d84af00e4016490b8ee8b1c51780546ad6016b7
```


### Check app status
Now, let's check your app state in the contract
```bash
~ $ ./fluence status \
            --eth_url          http://207.154.240.52:8545 \
            --contract_address 0xe9bbe60d525c7c5d4f3d85036f3ea23003879106 \
            --app_id           <your app id here>
```

The output will be in JSON, and look similar to the following:
```json
{
  "apps": [
    {
      "app_id": "<your app id here>",
      "storage_hash": "<swarm hash>",
      "storage_receipt": "0x0000000000000000000000000000000000000000000000000000000000000000",
      "cluster_size": 4,
      "owner": "<your ethereum address>",
      "pin_to_nodes": [],
      "cluster": {
        "genesis_time": 1549353504,
        "node_ids": [
          "0x5ed7aaada4bd800cd4f5b440f36ccece9c9e4542f9808ea6bfa45f84b8198185",
          "0xb557bb40febb7484393c1c99263b763d1caf6b6c83bc0a9fd6c084d2982af763",
          "0xac72ccc7886457c3f7da048e184b8b8a43f99c77950e7bb635b6cb3aeb3869fe",
          "0x9251dd451f4bd9f412173cc21279afc8d885312eb1c21828134ba9095da8306b",
        ],
        "ports": [
          25001
        ]
      }
    }
  ],
  "nodes": [
    {
      "validator_key": "0x5ed7a87da4bd800cd4f5b440f36ccece9c9e4542f9808ea6bfa45f84b8198185",
      "tendermint_p2p_id": "0x6c03a3fe792314f100ac8088a161f70bd7d257b1",
      "ip_addr": "43.32.21.10",
      "next_port": 25003,
      "last_port": 25099,
      "owner": "0x5902720e872fb2b0cd4402c69d6d43c86e973db7",
      "is_private": false,
      "app_ids": [
        1,
        2,
        6
      ]
    },
    "<3 more nodes here>"
  ]
}
```

You can also use interactive mode instead of default by supplying `--interactive` flag:
```bash
./fluence status \
            --eth_url          http://207.154.240.52:8545 \
            --contract_address 0xe9bbe60d525c7c5d4f3d85036f3ea23003879106 \
            --app_id           <your app id here> \
            --interactive
```

You can press `q` to exit it.

Your backend now is successfully deployed! You can proceed to access your code from a web browser.

## Frontend
For this part, you will need installed `npm`. Please refer to [npm docs](https://www.npmjs.com/get-npm) for installation instructions.

### Preparing web app
Let's clone a simple web app template:
```bash
~ $ git clone https://github.com/fluencelabs/frontend-template
~ $ cd frontend-template
~/frontend-template $ 
```

There are just three files (except for README, LICENSE and .gitignore):
- `package.json` that declares needed dependencies
- `webpack.config.js` needed for the webpack to work
- `index.js` that imports `fluence` js library and shows how to connect to a cluster

Let's take a look at `index.js`:
```javascript
import * as fluence from "fluence";

// address to Fluence contract in Ethereum blockchain. Interaction with blockchain created by MetaMask or with local Ethereum node
let contractAddress = "0x5faa7b8d290407460e0ec8585b2712acf27290f9";

// url of Ethereum node
let ethUrl = "http://207.154.240.52:8545"

// application to interact with that stored in Fluence contract
let appId = "1"; // <<--- PUT YOUR APP ID HERE

window.fluence = fluence;

// creates a session between client and backend application
fluence.connect(contractAddress, appId, ethUrl).then((s) => {
		console.log("Session created");
		window.session = s;
});

// gets a result and logs it
window.logResultAsString = function (request) {
	request.result().then((r) => console.log(r.asString()))
};
```

What this code does, line-by-line:
1. Imports `fluence` js library to be able to use it
2. Sets `contractAddress` variable to the address of Fluence contract
3. Sets `ethUrl` to the url of Ethereum node.
4. Sets `appId` to the desired appId. Put yours here.
5. Saves `fluence` to `window` property, so you can access it in Developer Console and experiment.
6. Calls the `connect` function, creating a connection to the Fluence cluster hosting your backend.
7. Saves session to `window.fluenceSession`, so it can be accessed later.
8. And final three lines define a helper function `logResultAsString` that's useful for printing out results.

**Make sure you have changed `appId` to your actuall appId.**

### Running and using
To install all dependencies, compile and run the application, run in the terminal:
```bash
~/frontend-template $ npm install
~/frontend-template $ npm run start
> frontend-template@1.0.0 start /private/tmp/frontend-template
> webpack-dev-server

ℹ ｢wds｣: Project is running at http://localhost:8080/
...
```

Now you can open http://localhost:8080/ in your browser. Then open Developer Console, and wait for the following messages:
```
...
Connecting web3 to http://46.101.213.180:8545
...
Session created
```

Now let's send a request and see if we get our greeting!
```javascript
let result = session.invoke("myName");
<undefined>
logResultAsString(result);
<undefined>
Hello, world! From user myName
```

Yaaay! Everything works.
