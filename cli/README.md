- [Fluence CLI](#fluence-cli)
- [Requirements](#requirements)
- [Installation](#installation)
- [Usage](#usage)
- [Usage examples](#usage-examples)
  - [Register a node](#register-a-node)
  - [Publish an app](#publish-an-app)
    - [Waiting for an app to be deployed or enqueued](#waiting-for-an-app-to-be-deployed-or-enqueued)
  - [Delete an app](#delete-an-app)
  - [Retrieve Fluence network state as JSON](#retrieve-fluence-network-state-as-json)
    - [Filtering status](#filtering-status)
- [Tips and tricks](#tips-and-tricks)
  - [Waiting for an Ethereum node to sync](#waiting-for-an-ethereum-node-to-sync)
  - [Waiting for a transaction to be included in a block](#waiting-for-a-transaction-to-be-included-in-a-block)
  - [Interactive status](#interactive-status)
  - [Authorization and private keys](#authorization-and-private-keys)
    - [Keystore JSON file](#keystore-json-file)
    - [Private key](#private-key)
    - [Password](#password)
    - [No authorization](#no-authorization)

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
            --tendermint_node_id    5e4eedba85fda7451356a03caffb0716e599679b \            
            --account               0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --base64_tendermint_key \
            --secret_key            0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133 \
            --wait_syncing \
            --api_port              25000 \
            --capacity              10
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
- `--api_port 25000` specifies the main port of the Fluence node, so other nodes and users know where to connect 
- `--capacity 10` limits number of apps that could be run on the node by 10 


### Publish an app
To deploy your app on Fluence network, you must upload it to Swarm and publish hash to smart-contract. The simplest way to achieve that is to use CLI command `publish`.

The following command will publish app `counter.wasm`.
```
./fluence publish \
            --code_path        fluence/vm/examples/counter/target/wasm32-unknown-unknown/release/deps/counter.wasm \            
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

Please refer to [Fluence Rust SDK](../backend-sdk/README.md) to get information about developing apps with Fluence.

#### Waiting for an app to be deployed or enqueued
You can pass `--wait` option to `publish` command, and CLI will wait until transaction is included in a block, and then provide you with information about app deployment status and app id.
Deployed application:
```bash
[1/3]   Application code uploaded. ---> [00:00:00]
swarm hash: 0x585114171e6b1639af3e9a4f237d8da6d1c5624b235cb45c062ca5d89a151cc2
[2/3]   Transaction publishing app was sent. ---> [00:00:00]
  tx hash: 0xf62a8823e95804bf1f8d3832c0c49d44c7c138c1a541f9f5c0dbe7cd34056f40
[3/3]   Transaction was included. ---> [00:00:00]
App deployed.
   app id: 1
  tx hash: 0xf62a8823e95804bf1f8d3832c0c49d44c7c138c1a541f9f5c0dbe7cd34056f40
```

Enqueued:
```bash
[1/3]   Application code uploaded. ---> [00:00:00]
swarm hash: 0x585114171e6b1639af3e9a4f237d8da6d1c5624b235cb45c062ca5d89a151cc2
[2/3]   Transaction publishing app was sent. ---> [00:00:00]
  tx hash: 0x4e87150da0e273fc7d44324e6843ba9abb91413f6fb6fdce67b7f7d4a1dd320a
[3/3]   Transaction was included. ---> [00:00:00]
App enqueued.
   app id: 2
  tx hash: 0x4e87150da0e273fc7d44324e6843ba9abb91413f6fb6fdce67b7f7d4a1dd320a
```

### Delete an app
If you want to delete your app from smart contract, you can use `delete_app` command.

The following will delete app with id `0x0000000000000000000000000000000000000000000000000000000000000002`. App id could be retrieved either from status (see below) or from smart-contract.

```
./fluence delete_app \            
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
./fluence status
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

#### Filtering status
If you need to select specific information from status, you can use different filters:
```bash
OPTIONS:
    ...
    -a, --app_id <app_id>                   Filter nodes and apps by app id
    -i, --node_ip <ip address>              Filter nodes by IP address
    -o, --owner <eth address>               Filter nodes and apps owned by this Ethereum address
    -K, --tendermint_key <key>              Filter nodes and apps by Tendermint validator key (node id)
    -f, --filter_mode <and|or>              Logical mode of the filter [default: and]
```

Filters work in both JSON and interactive modes. Here's an example of how they can be used:
```bash
./fluence status \            
            --tendermint_key 0xb5575140febb7484393c1c99263b763d1caf6b6c83bc0a9fd6c084d2982af763 \
            --node_ip 43.32.21.10 \
            --filter_mode or
```

This will display all nodes with id `0xb5575140febb7484393c1c99263b763d1caf6b6c83bc0a9fd6c084d2982af763`, all nodes with ip `43.32.21.10` and all apps hosted by these nodes.

Note `--filter_mode or`, it directs CLI to match all nodes and apps that satisfy any of specified filters. You can also pass `--filter_mode and`:
```bash
./fluence status \            
            --tendermint_key 0xb5575140febb7484393c1c99263b763d1caf6b6c83bc0a9fd6c084d2982af763 \
            --node_ip 43.32.21.10 \
            --filter_mode and
```

And you will get only node with id `0xb5575140febb7484393c1c99263b763d1caf6b6c83bc0a9fd6c084d2982af763` **and** IP `43.32.21.10`, if there is one, and all apps hosted by that node.

## Tips and tricks
### Waiting for an Ethereum node to sync
There is a flag `--wait_syncing` that, when supplied, will make CLI to wait until your Ethereum node is fully synced. It works by querying `eth_syncing` until it returns `false`. 

This is handy when you don't want to manually check if Ethereum node is synced.

### Waiting for a transaction to be included in a block
There is a flag `--wait` than, when supplied, will make CLI to wait until the sent transaction is included in a block. It also parses Ethereum events (logs) related to issued command, and will print out some useful information on finish. 

For example, when you `publish` your app, it will tell you the `appID` and if it's been deployed immediatly or enqueued to wait for enough available nodes. 

This is easier that manually checking `status` after every command.

Note, however, that if you're using Ethereum node in a **light mode**, it can take a while until light node realizes transaction was included in a block. It can take up to several minutes (sometimes up to 10-15 minutes), so it requires some patience.

### Specify contract address
There is a flag `--contract_address` to use all commands to interact with non-default Fluence smart contract. You can deploy [Fluence contract](../bootstrap/contracts/Network.sol) on your own and use CLI like this:
```bash
./fluence <command>
            ...
            --contract_address 0x9995882876ae612bfd829498ccd73dd962ec950a \
            ...
``` 

### Interactive status
If reading raw JSON in `status` isn't the best option for you, you can use interactive status:
```bash
./fluence status --interactive
```

It's just a status viewer, but it will gain more functionality in the future.

### Authorization and private keys
There are several ways to provide authorization details for your Ethereum account to Fluence CLI: via keystore JSON file, private key or password for your wallet in Ethereum node.

#### Keystore JSON file
***This is the most secure way to provide your credentials, so it's preffered over other options***

That's how Geth and a few other tools export private keys. The file looks like this:
```json
{"address":"c2d7cf95645d33006175b78989035c7c9061d3f9",
  "crypto":{
    "cipher":"aes-128-ctr",
    "ciphertext":"0f6d343b2a34fe571639235fc16250823c6fe3bc30525d98c41dfdf21a97aedb",
    "cipherparams":{
      "iv":"cabce7fb34e4881870a2419b93f6c796"
    },
    "kdf":"scrypt",
    "kdfparams": {
      "dklen":32,
      "n":262144,
      "p":1,
      "r":8,
      "salt":"1af9c4a44cf45fe6fb03dcc126fa56cb0f9e81463683dd6493fb4dc76edddd51"
    },
    "mac":"5cf4012fffd1fbe41b122386122350c3825a709619224961a16e908c2a366aa6"
  },
  "id":"eddd71dd-7ad6-4cd3-bc1a-11022f7db76c",
  "version":3
}
```

It's a private key encrypted with user password. You can use it with Fluence CLI like this: 
```bash
./fluence SUBCOMMAND
          ...
          --account            0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
          --keystore           ~/Library/Ethereum/keystore/UTC--2017-03-03T13-24-07.826187674Z--4e6cf0ed2d8bbf1fbbc9f2a100602ceba4bf1319 \
          --passowrd           my_secure_passw0rd
```

For example, with `delete_app`

```bash
./fluence delete_app \            
            --account          0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --app_id           0x0000000000000000000000000000000000000000000000000000000000000002 \
            --keystore         ~/Library/Ethereum/keystore/UTC--2017-03-03T13-24-07.826187674Z--4e6cf0ed2d8bbf1fbbc9f2a100602ceba4bf1319 \
            --password         my_secure_passw0rd \
            --deployed
```

Decrypted private key will be used to sign transaction in offline mode, so **your key is never sent through network.**

#### Private key
Other option is to provide unencrypted private key, like in most examples above:
```bash
./fluence SUBCOMMAND
          ...
          --account            0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
          --secret_key         4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7 
```

Since private key isn't encrypted, no password is required.

Private key will be used to sign transaction in offline mode, so **your key is never sent through network.**

#### Password
In case you have a **trusted** Ethereum node with an imported wallet, Fluence CLI can delegate signing a transaction to the Ethereum node. This can be done like this:
```bash
./fluence SUBCOMMAND
          ...
          --account            0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
          --password           my_secure_passw0rd
```

`personal_sendTransaction` will be used to send transaction. It means **your password will be sent over network** to the Ethereum node, and the node will sign and send transaction all by itself. It's preffered to use keystore or private key options instead of providing just a password.

Note, that **your account is not unlocked before, in, or after that operation.**

#### No authorization
It's also possible to avoid providing any credentials:
```bash
./fluence SUBCOMMAND
          ...
          --account            0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d
```

In that case, transaction is sent to Ethereum node via `eth_sendTransaction` unsigned, so it's expected that there is no authorization enabled on your node. **This option isn't secure and was meant to be used for testing purposes.**
