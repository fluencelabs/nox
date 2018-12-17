## Fluence Publisher

Current code deployment process to the Fluence network looks like this:

- Upload code to Swarm. Swarm will return the code hash.
- Publish the received hash to a default Fluence contract in a blockchain (ethereum as a default).

**Fluence Publisher** is a console automation utility that will take the code as an input, go through the publishing process and return a hash of transaction of publishing to the developer, if successful.

 ## Requirements
 
- a connection to the Ethereum node (including light client) that is able to broadcast a transaction
- Ethereum account with a balance sufficient to pay for gas
- address of the Swarm node

## Installation

Requirements: [`cargo`](https://doc.rust-lang.org/cargo/getting-started/installation.html) and [`npm`](https://www.npmjs.com/get-npm) are needed.

- clone repo and `cd` to `publisher` folder
- run `cargo build`
- go to `target/debug`
- there is an executable file `fluence` to work with

## Usage

To look at all possible arguments and options use `./fluence --help`:

```
Fluence CLI 0.1.0
Fluence Labs
Console utility for deploying code to fluence cluster

USAGE:
    fluence [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    add-to-whitelist    Adds an address to the whitelist of Fluence smart contract
    check               Verifies wasm file, issue warning for using functions from banned modules.
    help                Prints this message or the help of the given subcommand(s)
    publish             Publish code to ethereum blockchain
    register            Register solver in smart contract
    status              Get status of smart contract
```

You can use `./fluence [SUBCOMMAND] --help` to learn how to use commands.

