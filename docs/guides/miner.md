- [Fluence miner guide](#fluence-miner-guide)
  - [How mining works](#how-mining-works)
  - [How to become a Fluence miner](#how-to-become-a-fluence-miner)
    - [Deploy Fluence node to a cloud](#deploy-fluence-node-to-a-cloud)
    - [Deploy Fluence node locally](#deploy-fluence-node-locally)
  - [How to check if node is registered](#how-to-check-if-node-is-registered)
  - [Private nodes and application pinning](#private-nodes-and-application-pinning)
    - [Private nodes](#private-nodes)
    - [Application pinning](#application-pinning)
      - [Mixing pinning and matching](#mixing-pinning-and-matching)
  - [How to remove a node from the Fluence smart contract](#how-to-remove-a-node-from-the-fluence-smart-contract)
  - [Tips and tricks](#tips-and-tricks)

# Fluence miner guide
Being a miner in Fluence network means that you will provide you computation power to host decentralized backends. In order to do that, you will need just a few things:
1. A computer to run a Fluence node and workers on it
2. Installed and running Docker and `docker-compose`
3. Installed python2 pip
4. An Ethereum light-client or full node connected to Kovan testnet
5. A Kovan Ethereum wallet topped up with some ETH to submit transactions

## How mining works
The process is as follows:
You run a Docker image with Fluence node on your computer, then you register that node within Fluence smart contract by using Fluence CLI. On registration, you specify max number of backends you wish to host, so your node doesn't run out of resources.

After you registered your node within smart contract, it starts waiting to be included in an application cluster. That will happen when someone publishes an app and the Fluence smart contract will match that app to your node, and then send an Ethereum event that your node is listening to.

When node receives an event stating it's a part of an application cluster now, it will download application code from Swarm, and then run a Docker image with Fluence worker hosting that code.

## How to become a Fluence miner
The first step is to deploy a Fluence node. Deploy is automated, you can get scripts by cloning [Fluence repo](https://github.com/fluencelabs/fluence):
```
git clone https://github.com/fluencelabs/fluence
```

### Deploy Fluence node to a cloud
It's an automated task, you will only need to specify your cloud instance IP addresses and your Ethereum wallet, and then run a (Fabric)[https://github.com/fabric/fabric] script.

First, let's install Fabric:
```bash
# make use you're using python2
$ pip --version
pip 18.0 from <...> (python 2.7)
$ pip install Fabric==1.14.1
```

It should install fabric 1.14.1. **Be careful not to install Fabric 2 as it's not current supported.**

Next, open [fluence/tools/deploy/instances.json](../../tools/deploy/instances.json) in your favorite text editor, and modify `config`:
```json
{
    "<ip1>": {
        "owner": "<owner-account1>",
        "key": "<secret-key1>",
        "ports": "<start-port1:end-port1>"
    },
    "<ip2>": {
        "owner": "<owner-account2>",
        "key": "<secret-key2>",
        "ports": "<start-port2:end-port2>"
    }
}
```

You can specify here several nodes, but for the sake of example, let's continue with a single node. After filling info in deploy_config.json, it should look similar to this:

```json
{
    "53.42.31.20": {
        "owner": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
        "key": "4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7",
        "ports": "25000:25099"
    }
}
```

You can also change user running setup commands on cloud instance by `env.user` to desired username.
```python
# Set the username
env.user = "root"
```

Now, let's deploy Fluence node along with Parity and Swarm containers:

```bash
$ fab deploy
```

At the end of the successful deployment you should see something like the following:
```
[53.42.31.20] out: CONTRACT_ADDRESS=0x45cc7b68406cca5bc36b7b8ce6ec537eda67bc0b
[53.42.31.20] out: NAME=fluence-node-1
[53.42.31.20] out: PORTS=25000:25099
[53.42.31.20] out: HOST_IP=53.42.31.20
[53.42.31.20] out: EXTERNAL_HOST_IP=53.42.31.20
[53.42.31.20] out: OWNER_ADDRESS=0x00a329c0648769a73afac7f9381e08fb43dbea72
[53.42.31.20] out: CONTRACT_ADDRESS=0x45cc7b68406cca5bc36b7b8ce6ec537eda67bc0b
[53.42.31.20] out: STATUS_PORT=25400
...
[53.42.31.20] out: Node container is started.
[53.42.31.20] out: CURRENT NODE = 1
[53.42.31.20] out: TENDERMINT_KEY=YAa6x36acTUIemAoejyplm+cUKe5rhTC1ArMPLMfvFY=
[53.42.31.20] out: START_PORT=25000
[53.42.31.20] out: LAST_PORT=25099
```

Here you have some useful information that you can use with Fluence CLI.

At the very end, you should see something like this:
```
[53.42.31.20] out: [1/2]   Node synced. ---> [00:00:00]
[53.42.31.20] out: [2/2]   Node added. ---> [00:00:00]
[53.42.31.20] out: Node registered. Submitted transaction: 0x51e1dcf1cf20e136ce66d240e9897c329894c1084e55b13a4fab751fbaced981
```

That's the result of script registering your node within Fluence smart contract. You can see it in [compose.sh](../../tools/deploy/scripts/compose.sh):
```bash
./fluence register \
        --node_ip           $EXTERNAL_HOST_IP \
        --tendermint_key    $TENDERMINT_KEY \
        --contract_address  $CONTRACT_ADDRESS \
        --account           $OWNER_ADDRESS \
        --secret_key        $PRIVATE_KEY \
        --start_port        $START_PORT \
        --last_port         $LAST_PORT \
        --wait_syncing \
        --base64_tendermint_key
```

All environment variables used in the command are listed in the useful information above, so you can easily use them with Fluence CLI.

### Deploy Fluence node locally
If you wish to use your local computer to host Fluence node, you can do that by running [kovan-compose.sh](../../tools/deploy/scripts/kovan-compose.sh) like this:
```
# ./kovan-compose.sh <external-ip> <owner-address> <private-key> <start-port:end-port>
./kovan-compose.sh 53.42.31.20 0x00a329c0648769a73afac7f9381e08fb43dbea72 4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7 25000:25099
```

## How to check if node is registered
You can use [Fluence CLI](../../cli/README.md) to query current state of network in Fluence smart contract like this:
```bash
./fluence status --contract_address 0x45cc7b68406cca5bc36b7b8ce6ec537eda67bc0b
```
where `0x45cc7b68406cca5bc36b7b8ce6ec537eda67bc0b` is the Fluence smart contract address. 

**Note:** if your Ethereum node is remote, you can specify it's address via `--eth_url` option, like this:
```bash
./fluence status --eth_url https://53.42.31.20:8545 --contract_address 0x45cc7b68406cca5bc36b7b8ce6ec537eda67bc0b
```

You should see your Ethereum address under `owner` in `nodes` list. Similar to this:
```json
{
  "apps": [],
  "nodes": [
    {
      "id": "0x6006bac77e9a7135087a60287a3ca9966f9c50a7b9ae14c2d40acc3cb31fbc56",
      "tendermint_key": "0x4b4432951f27c2e9a29017b3b6dee46a2e08c3a2",
      "ip_addr": "53.42.31.20",
      "next_port": 25001,
      "last_port": 25099,
      "owner": "0x00a329c0648769a73afac7f9381e08fb43dbea72",
      "is_private": false,
      "clusters_ids": []
    }
  ]
}
```

Please refer to Fluence CLI [README](../../cli/README.md) for more info on installation and usage.

## Private nodes and application pinning
### Private nodes
By default, after you register a node in the Fluence smart contract, any matching code could be deployed on it. So the node is publici by default.

But if you want to host only your app, there is a way to mark node **private**. That could be to be sure about what apps you're hosting, what's their workload or just to be sure _your_ app gets enough resources to be ran. 

When a node is marked as **private**, only your apps can be deployed on that node. This is determined by the Ethereum address that sent the transaction for node registration and for application deployment. 

The following command will register a private node:
```bash
./fluence register \
            --private \
            --node_ip               85.82.118.4 \
            --tendermint_key        1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= \
            --tendermint_node_id    5e4eedba85fda7451356a03caffb0716e599679b \
            --contract_address      0x9995882876ae612bfd829498ccd73dd962ec950a \
            --account               0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --base64_tendermint_key \
            --secret_key            0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133 \
            --wait_syncing \
            --start_port            25000 \
            --last_port             25010
```

As suggested by it's name, it's the `--private` flag what's making the registered node a private one.

Now, only apps [published](backend.md) to Fluence smart contract from your Ethereum address can get deployed on your nodes. **But these apps need to specify nodes they should be hosted on**, that's called **application pinning**.

### Application pinning
So, since your nodes are [private](#private-nodes), the application publishing process changes a little: you need to specify IDs of the nodes where you want the application to be hosted. With CLI, that's pretty easy. Assuming you have registered 4 private nodes, and their IDs are:
- `1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM=`
- `QYLcTI9uChtEVdXaR9+WgMlxJ1AVVuroe5Jyay+epbI=`
- `+l30LsCMPGeL6/YIwKg8RWOhS55PLIAZT05D11GJAxY=`
- `xkaW9SpCJjfzLABM+1B6PTh54qIwvgsHCkOe3VULYbE=`

All you have to do is to provide these node ids to `--pin_to` flag as a space-separated list:
```bash
./fluence publish \
            --code_path        fluence/vm/examples/counter/target/wasm32-unknown-unknown/release/deps/counter.wasm \
            --contract_address 0x9995882876ae612bfd829498ccd73dd962ec950a \
            --account          0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --cluster_size     4 \
            --secret_key       0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133 \
            --pin_to           1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= \
                               QYLcTI9uChtEVdXaR9+WgMlxJ1AVVuroe5Jyay+epbI= \
                               +l30LsCMPGeL6/YIwKg8RWOhS55PLIAZT05D11GJAxY= \
                               xkaW9SpCJjfzLABM+1B6PTh54qIwvgsHCkOe3VULYbE= \
            --base64
```

_Note that you can pin your app to your both **private** and **public** nodes._ Marking node as private just prevents random apps to be deployed on that node.

#### Mixing pinning and matching
The command above is a perfect match, it's pinning the app to four nodes, and also specifies a cluster size of four, so every node hosting the app is directly pinned to it. 

But what if your app requires more nodes than you wish to register? In that case, you can pin your app to any number of nodes, be it just one or a few, and specify a cluster size that you need. Fluence smart contract will then match your app both against your pinned nodes and available public nodes. The command is almost the same, except the `--cluster_size` requires eight nodes.
```bash
./fluence publish \
            --code_path        /Users/folex/Development/fluence/vm/examples/counter/target/wasm32-unknown-unknown/release/deps/counter.wasm \
            --contract_address 0x9995882876ae612bfd829498ccd73dd962ec950a \
            --account          0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --cluster_size     8 \
            --secret_key       0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133 \
            --pin_to           1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= \
                               QYLcTI9uChtEVdXaR9+WgMlxJ1AVVuroe5Jyay+epbI= \
                               +l30LsCMPGeL6/YIwKg8RWOhS55PLIAZT05D11GJAxY= \
                               xkaW9SpCJjfzLABM+1B6PTh54qIwvgsHCkOe3VULYbE= \
            --base64
```

So, assuming contract has just four registered nodes and one app, the `./fluence status --contract_address 0x9995882876ae612bfd829498ccd73dd962ec950a` will look like this:
```json
{
  "apps": [
    {
      "app_id": "0x0000000000000000000000000000000000000000000000000000000000000001",
      "storage_hash": "0x585114171e6b1639af3e9a4f237d8da6d1c5624b235cb45c062ca5d89a151cc2",
      "storage_receipt": "0x0000000000000000000000000000000000000000000000000000000000000000",
      "cluster_size": 8,
      "owner": "0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d",
      "pin_to_nodes": [
        "0xd46543202ce0af0d6a6a13df49bc027d8c34cebc3dd4e319e3a4282af24c8e33",
        "0x4182dc4c8f6e0a1b4455d5da47df9680c97127501556eae87b92726b2f9ea5b2",
        "0xfa5df42ec08c3c678bebf608c0a83c4563a14b9e4f2c80194f4e43d751890316",
        "0xc64696f52a422637f32c004cfb507a3d3879e2a230be0b070a439edd550b61b1"
      ],
      "cluster": null
    }
  ],
  "nodes": [
    {
      "id": "0xd46543202ce0af0d6a6a13df49bc027d8c34cebc3dd4e319e3a4282af24c8e33",
      "tendermint_key": "0x5e4eedba85fda7451356a03caffb0716e599679b",
      "ip_addr": "85.82.118.4",
      "next_port": 25000,
      "last_port": 25010,
      "owner": "0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d",
      "is_private": true,
      "clusters_ids": []
    },
    {
      "id": "0x4182dc4c8f6e0a1b4455d5da47df9680c97127501556eae87b92726b2f9ea5b2",
      "tendermint_key": "0x5e4eedba85fda7451356a03caffb0716e599679b",
      "ip_addr": "85.82.118.4",
      "next_port": 25000,
      "last_port": 25010,
      "owner": "0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d",
      "is_private": true,
      "clusters_ids": []
    },
    {
      "id": "0xfa5df42ec08c3c678bebf608c0a83c4563a14b9e4f2c80194f4e43d751890316",
      "tendermint_key": "0x5e4eedba85fda7451356a03caffb0716e599679b",
      "ip_addr": "85.82.118.4",
      "next_port": 25000,
      "last_port": 25010,
      "owner": "0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d",
      "is_private": true,
      "clusters_ids": []
    },
    {
      "id": "0xc64696f52a422637f32c004cfb507a3d3879e2a230be0b070a439edd550b61b1",
      "tendermint_key": "0x5e4eedba85fda7451356a03caffb0716e599679b",
      "ip_addr": "85.82.118.4",
      "next_port": 25000,
      "last_port": 25010,
      "owner": "0x4180rfc65d613ba7e1a385181a219f1dbfe7bf11d",
      "is_private": true,
      "clusters_ids": []
    }
  ]
}
```

You see app's `cluster` is `null`, that means it's not deployed yet, waiting for enough nodes to be available.

## How to remove a node from the Fluence smart contract
If you shut down your node, it's not available anymore for any reason, or you just want to stop it hosting apps, you can remove the node from the Fluence smart contract like this:
```bash
./fluence delete_node \
            --contract_address 0x9995882876ae612bfd829498ccd73dd962ec950a \
            --account          0x4180fc65d613ba7e1a385181a219f1dbfe7bf11d \
            --secret_key       0xcb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133 \
            --tendermint_key   1GVDICzgrw1qahPfSbwCfYw0zrw91OMZ46QoKvJMjjM= \
            --base64_tendermint_key
```

## Tips and tricks
CLI has some neat features not described in that guide:
- Wait until your Ethereum is fully synced
- Wait until the transaction is included in a block, printing additional info from contract
- Look at smart contract `status` via interactive command-line table viewer

All these features are described in [CLI's readme](../../cli/README.md#tips-and-tricks), so take a look!
