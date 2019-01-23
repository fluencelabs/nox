- [Fluence miner guide](#fluence-miner-guide)
  - [How mining works](#how-mining-works)
  - [How to become a Fluence miner](#how-to-become-a-fluence-miner)
    - [Deploy Fluence node to a cloud](#deploy-fluence-node-to-a-cloud)
    - [Deploy Fluence node locally](#deploy-fluence-node-locally)

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
```
# make use you're using python2
$ pip --version
pip 18.0 from <...> (python 2.7)
$ pip install Fabric==1.14.1
```

It should install fabric 1.14.1. **Be careful not to install Fabric 2 as it's not current supported.**

Next, open [fluence/tools/deploy/fabfile.py](../../tools/deploy/fabfile.py) in your favorite text editor, and modify `info`:
```
info = {'<ip1>': {'owner': '<eth address1>', 'key': '<private key1>', 'ports': '<from>:<to>'},
        '<ip2>': {'owner': '<eth address2>', 'key': '<private key2>', 'ports': '<from>:<to>'}}
```

You can specify here several nodes, but for the sake of example, let's continue with a single node. After filling info in fabfile.py, it should look similar to this:

```
info = {
    '53.42.31.20':
        {
            'owner': '0x00a329c0648769a73afac7f9381e08fb43dbea72',
            'key': '4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7',
            'ports': '25000:25099'
        }
    }
```

You can also change user running setup commands on cloud instance by `env.user` to desired username.
```
# Set the username
env.user = "root"
```

Now, let's deploy Fluence node along with Parity and Swarm containers:

```
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
[53.42.31.20] out: PRIVATE_KEY=4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7
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
```
./fluence register $EXTERNAL_HOST_IP $TENDERMINT_KEY $OWNER_ADDRESS $CONTRACT_ADDRESS -s $PRIVATE_KEY --wait_syncing --start_port $START_PORT --last_port $LAST_PORT --base64_tendermint_key
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
```
./fluence status 0x45cc7b68406cca5bc36b7b8ce6ec537eda67bc0b
```
where `0x45cc7b68406cca5bc36b7b8ce6ec537eda67bc0b` is the Fluence smart contract address. 

**Note:** if your Ethereum node is remote, you can specify it's address via `--eth_url` option, like this:
```
./fluence status --eth_url https://53.42.31.20:8545 0x45cc7b68406cca5bc36b7b8ce6ec537eda67bc0b
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
    },
```

Please refer to Fluence CLI [README](../../cli/README.md) for more info on installation and usage.