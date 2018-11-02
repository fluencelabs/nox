## Fluence Publisher

Current code deployment process to the Fluence network looks like this:

- Upload code to Swarm. Swarm will return the code hash.
- Publish the received hash to a default “deployer” contract in a blockchain (ethereum as a default).

**Fluence Publisher** is a console automation utility that will take the code as an input, go through the publishing process and return a hash of transaction of publishing to the developer, if successful.

 ## Requirements
 
- address of Ethereum node that can do transactions
- Ethereum account with enough gas for the transaction
- address of public remote or local Swarm node

## Installation

Requirements: [`rust`](https://www.rust-lang.org/install.html) and [`cargo`](https://doc.rust-lang.org/cargo/getting-started/installation.html) are needed.

- clone repo and `cd` to `publisher` folder
- run `cargo build`
- go to `target/debug`
- there is executable file `publisher` to work with

## Usage

To look at all possible arguments and options use `./publisher --help`:

```
USAGE:
    publisher [OPTIONS] <path> <contract_address> --account <account>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --account <account>              ethereum account without `0x`
    -c, --cluster_size <cluster_size>    cluster's size that needed to deploy this code [default: 3]
    -e, --eth_url <eth_url>              http address to ethereum node [default: http://localhost:8545/]
    -p, --password <password>            password to unlock account in ethereum client
    -s, --swarm_url <swarm_url>          http address to swarm node [default: http://localhost:8500/]

ARGS:
    <path>                path to compiled `wasm` code
    <contract_address>    deployer contract address without `0x`
```
