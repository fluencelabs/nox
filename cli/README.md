## Fluence CLI

Fluence CLI is an automation tool for tasks of app management (deployment and deletion), computation resource sharing (node registration), and monitoring Fluence network state (status). See usage for more info.

## Requirements

CLI assumes running Ethereum and Swarm on `http://localhost:8545/` and `http://localhost:8500/` respectively. Use `--eth_url` and `--swarm_url` to specify actual addresses as you need.

Please note, that your Ethereum account should have sufficient funds for issuing transactions to smart-contract. It's only for transaction fees, Fluence itself doesn't currently charge miners or developers. That could change in the future, for example when miners' deposits are implemented.

Also, take a look at [deployment scripts](../tools/deploy/README.md), they will assist you in running Swarm, Ethereum and Fluence nodes.


## Installation

Requirements: [`cargo`](https://doc.rust-lang.org/cargo/getting-started/installation.html) and [`npm`](https://www.npmjs.com/get-npm) are needed.

- clone this repo and `cd` to `cli` folder
- run `cargo build`
- go to `target/debug`
- there is an executable file `fluence` to work with

## Usage

To look at all possible arguments and options use `./fluence --help`:

You can use `./fluence [SUBCOMMAND] --help` to learn how to use commands.

## Usage examples
### Register a node
To provide your computation resources to Fluence network, you need to register your computer within smart-contract. The simplest way to do that is through CLI.
The following command will register a node:
```
./fluence register \
            --node_ip               85.82.118.4 \
            --tendermint_key        1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= \
            --tendermint_node_id    5e4eedba85fda7451356a03caffb0716e599679b= \
            --contract_address      0x9995882876ae612bfd829498ccd73dd962ec950a \
            --account               0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --base64_tendermint_key \
            --secret_key            0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133 \
            --wait_syncing \
            --start_port            25000 \
            --last_port             25010
```

Parameters are:
- advertised address `85.82.118.4`, please note that this address should be available from Internet as it will be used to connect with other workers in a future cluster
- Tendermint key (used to identify node) `1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM=`, note base64 format
    - flag `--base64_tendermint_key` passed so tendermint key is treated as base64-encoded as opposed to hex-encoded
    - currently, Tendermint key can be found in logs of `fluencelabs/node` Docker container
    - note that key should be unique, i.e. you can't register several nodes with the same key
- Tendermint p2p node ID `5e4eedba85fda7451356a03caffb0716e599679b` is needed to securely connect nodes in Tendermint cluster
- `0x9995882876ae612bfd829498ccd73dd962ec950a` is a contract address, register transaction will be sent there
- `0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d` will be used as Ethereum account for issuing transactions. _Use your Ethereum account here_
- `--secret_key 0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133` denotes an Ethereum private key, used for offline transaction signing. _Use your Ethereum private key here_
    - using `--password` is possible instead of private key, but private key is preferred
- `--wait_syncing` so CLI waits until Ethereum node is fully synced
- `--start_port 25000` and `--last_port 25010` denote ports where apps (workers) will be hosted. 25000:25010 is inclusive, so 10 workers could be started on such a node


### Publish app
To deploy your app on Fluence network, you must upload it to Swarm and publish hash to smart-contract. The simplest way to achieve that is to use CLI command `publish`.

The following command will publish app `counter.wasm`.
```
./fluence publish \
            --code_path        fluence/vm/examples/counter/target/wasm32-unknown-unknown/release/deps/counter.wasm \
            --contract_address 0x9995882876ae612bfd829498ccd73dd962ec950a \
            --account          0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --cluster_size     4 \
            --secret_key       0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133 \
            --pin_to           1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= \
            --base64
```

Interesting bits:
- `--cluster_size 4` requires cluster of 4 workers to host this app
- `--pin_to 1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= --base64` requires that one of the node in cluster must be `1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM=`
    - note that to be used in `pin_to` node must be already registered in smart-contract
- `fluence/vm/examples/counter/target/wasm32-unknown-unknown/release/deps/counter.wasm` is just an example path and doesn't exist in project
    - however, you can build it by issuing `sbt vm-counter/compile` in project root

Please refer to [Fluence Rust SDK](../sdk/rust/README.md) to get information about developing apps with Fluence.

### Delete app
If you want to delete your app from smart contract, you can use `delete_app` command.

The following will delete app with id `0x0000000000000000000000000000000000000000000000000000000000000002`. App id could be retrieved either from status (see below) or from smart-contract.

```
./fluence delete_app \
            --contract_address 0x9995882876ae612bfd829498ccd73dd962ec950a \
            --account          0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --app_id           0x0000000000000000000000000000000000000000000000000000000000000002 \
            --secret_key       4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7 \
            --deployed
```

Note `--deployed` at the end. It means that app is deployed and cluster hosting it should be deleted as well. Without that flag, app would be removed only if there is no assigned cluster (i.e., app is not yet deployed).

See below on how to know if your app is deployed.

### Retrieve Fluence network state as JSON
To inspect what apps are in queue, what clusters are working out there and what nodes are participating in them, you can use `status` command as follows:
```
./fluence status --contract_address 0x9995882876ae612bfd829498ccd73dd962ec950a
```

The results will be in JSON and should resemble the following
```json
{
  "apps": [
    {
      "app_id": "0x0000000000000000000000000000000000000000000000000000000000000001",
      "storage_hash": "0xeb2a623210c080d0702cc520b790151861601c46d90179a6e8efe6bda8ac5477",
      "storage_receipt": "0x0000000000000000000000000000000000000000000000000000000000000000",
      "cluster_size": 5,
      "owner": "0x64b8f12d14925394ae0119466dff6ff2b021a3e9",
      "pin_to_nodes": [],
      "cluster": null
    },
    {
      "app_id": "0x0000000000000000000000000000000000000000000000000000000000000005",
      "storage_hash": "0xeb2a623210c080d0702cc520b790151861601c46d90179a6e8efe6bda8ac5477",
      "storage_receipt": "0x0000000000000000000000000000000000000000000000000000000000000000",
      "cluster_size": 5,
      "owner": "0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d",
      "pin_to_nodes": [
        "0x000000000000000000000000000000000000000000000000ef0f3348eb26bcc5",
        "0x0000000000000000000000000000000000000000000000009d0b83faa72e0948",
        "0x0000000000000000000000000000000000000000000000003b0222f266098324",
        "0x000000000000000000000000000000000000000000000000d4cbdb932f4294d6",
        "0x0000000000000000000000000000000000000000000000005aaa1fdc5999eacb"
      ],
      "cluster": {
        "genesis_time": 1547643251,
        "node_ids": [
          "0x000000000000000000000000000000000000000000000000ef0f3348eb26bcc5",
          "0x0000000000000000000000000000000000000000000000009d0b83faa72e0948",
          "0x0000000000000000000000000000000000000000000000003b0222f266098324",
          "0x000000000000000000000000000000000000000000000000d4cbdb932f4294d6",
          "0x0000000000000000000000000000000000000000000000005aaa1fdc5999eacb"
        ],
        "ports": [
          25000,
          25002,
          25004,
          25006,
          25008
        ]
      }
    }
  ]
}
```

Here you can see two apps, the first one is enqueued (cluster is `null`) and waiting for enough nodes to host it, and the second one is already hosted on top of 5 nodes. Second app also specifies all 5 nodes in `pin_to_nodes`, and you can see the same nodes in `cluster.node_ids`.

