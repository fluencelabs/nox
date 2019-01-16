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

- clone repo and `cd` to `cli` folder
- run `cargo build`
- go to `target/debug`
- there is an executable file `fluence` to work with

## Usage

To look at all possible arguments and options use `./fluence --help`:

You can use `./fluence [SUBCOMMAND] --help` to learn how to use commands.

## Usage examples
### Register a node
To provide your computation resources to Fluence network, you need to register your computer within smart-contract. The simplest way to do that is through CLI.
The following command will register a node with the following parameters:
- advertised address `85.82.118.4`, please note that this address should be available from Internet as it will be used to connect with other workers in a future cluster
- Tendermint key (also used to identify node) `1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM=`, note base64 format
    - flag `--base64_tendermint_key` passed so tendermint key is treated as base64-encoded as opposed to hex-encoded
    - currently, Tendermint key can be found in logs of `fluencelabs/node` Docker container
-  `0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d` will be used as Ethereum account for issuing transactions
- `0x9995882876ae612bfd829498ccd73dd962ec950a` is a contract address, register transaction will be sent there
- `--secret-key 0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133` denotes an Ethereum private key, used for offline transaction signing
- `--wait_syncing` so CLI waits until Ethereum node is fully synced
- `--start_port 25000` and `--last_port 25010` denote ports where apps (workers) will be hosted. 25000:25010 is inclusive, so 10 workers could be started on such a node
```
./fluence register 85.82.118.4 1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= 0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d 0x9995882876ae612bfd829498ccd73dd962ec950a --base64_tendermint_key --secret-key 0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133 --wait_syncing --start_port 25000 --last_port 25010
```

### Publish app
The following command will publish app `counter.wasm`. Interesting bits:
- `--cluster_size 4` requires cluster of 4 workers to host this app
- `--pin_to 1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= --base64` requires that one of the node in cluster must be `1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM=`
```
./fluence publish fluence/vm/examples/counter/target/wasm32-unknown-unknown/release/deps/counter.wasm 0x9995882876ae612bfd829498ccd73dd962ec950a 0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d --cluster_size 4 --secret-key 0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133 --pin_to 1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= --base64
```

### Delete app
The following will delete app with id `0x0000000000000000000000000000000000000000000000000000000000000002`. App id could be retrieved either from status (see below) or from smart-contract
Note `-D` at the end. It means that app is deployed and cluster hosting it should be deleted as well. Without that flag, app would be removed only if there is no assigned cluster (i.e., app is not yet deployed).
```
./fluence delete_app 0x9995882876ae612bfd829498ccd73dd962ec950a 0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d 0x0000000000000000000000000000000000000000000000000000000000000002 --secret-key 4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7 -D
```

