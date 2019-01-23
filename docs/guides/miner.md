## Fluence miner guide
Being a miner in Fluence network means that you will provide you computation power to host decentralized backends. In order to do that, you will need just a few things:
1. A computer to run a Fluence node and workers on it
2. Installed and running Docker
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
$ pip install fabric
```

It should install fabric 2.4 or later.

Next, open [fluence/tools/deploy/fabfile.py](../../tools/deploy/fabfile.py) in your favorite text editor, and modify `info`:
```
info = {'<ip1>': {'owner': '<eth address1>', 'key': '<private key1>'},
        '<ip2>': {'owner': '<eth address2>', 'key': '<private key2>'}}
```

You can specify here several nodes, but for the sake of example, let's continue with a single node. After filling info, it should look alike to the following

```
info = {'53.42.31.20': {'owner': '<eth address1>', 'key': '<private key1>'}}
```

### Deploy Fluence node locally